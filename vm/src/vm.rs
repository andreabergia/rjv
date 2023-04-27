use std::{cell::RefCell, collections::HashMap, rc::Rc};

use log::{debug, error};

use rjvm_reader::field_type::{BaseType, FieldType};

use crate::{
    call_frame::MethodCallResult,
    call_stack::CallStack,
    class::{ClassId, ClassRef},
    class_and_method::ClassAndMethod,
    class_manager::{ClassManager, ResolvedClass},
    class_path::ClassPathParseError,
    exceptions::MethodCallFailed,
    gc::ObjectAllocator,
    native_methods_registry::NativeMethodsRegistry,
    value::{ObjectRef, Value},
    vm_error::VmError,
};

#[derive(Debug, Default)]
pub struct Vm<'a> {
    /// Responsible for allocating and storing classes
    class_manager: ClassManager<'a>,

    /// Responsible for allocating objects
    object_allocator: ObjectAllocator<'a>,

    /// To model static fields, we will create one special instance of each class
    /// and we will store it in this map
    statics: HashMap<ClassId, ObjectRef<'a>>,

    /// Stores native methods
    pub native_methods_registry: NativeMethodsRegistry<'a>,

    pub printed: Vec<Value<'a>>, // Temporary, used for testing purposes
}

impl<'a> Vm<'a> {
    pub fn new() -> Self {
        let mut result: Self = Default::default();
        crate::native_methods_impl::register_natives(&mut result.native_methods_registry);
        result
    }

    pub fn extract_str_from_java_lang_string(
        &self,
        object: ObjectRef<'a>,
    ) -> Result<String, VmError> {
        let class = self.get_class_by_id(object.class_id)?;
        if class.name == "java/lang/String" {
            // In our JRE's rt.jar, the first fields of String is
            //    private final char[] value;
            if let Value::Array(_, array_ref) = object.get_field(0) {
                let string_bytes: Vec<u8> = array_ref
                    .borrow()
                    .iter()
                    .map(|v| match v {
                        Value::Int(c) => *c as u8,
                        _ => panic!("array items should be chars"),
                    })
                    .collect();
                let string = String::from_utf8(string_bytes).expect("should have valid utf8 bytes");
                return Ok(string);
            }
        }
        Err(VmError::ValidationException)
    }

    pub(crate) fn get_static_instance(&self, class_id: ClassId) -> Option<ObjectRef<'a>> {
        self.statics.get(&class_id).cloned()
    }

    pub fn append_class_path(&mut self, class_path: &str) -> Result<(), ClassPathParseError> {
        self.class_manager.append_class_path(class_path)
    }

    pub fn get_or_resolve_class(
        &mut self,
        stack: &mut CallStack<'a>,
        class_name: &str,
    ) -> Result<ClassRef<'a>, MethodCallFailed<'a>> {
        let class = self.class_manager.get_or_resolve_class(class_name)?;
        if let ResolvedClass::NewClass(classes_to_init) = &class {
            for class_to_init in classes_to_init.to_initialize.iter() {
                self.init_class(stack, class_to_init)?;
            }
        }
        Ok(class.get_class())
    }

    fn init_class(
        &mut self,
        stack: &mut CallStack<'a>,
        class_to_init: &ClassRef<'a>,
    ) -> Result<(), MethodCallFailed<'a>> {
        let static_instance = self.new_object_of_class(class_to_init);
        self.statics.insert(class_to_init.id, static_instance);
        if let Some(clinit_method) = class_to_init.find_method("<clinit>", "()V") {
            debug!("invoking {}::<clinit>()", class_to_init.name);
            self.invoke(
                stack,
                ClassAndMethod {
                    class: class_to_init,
                    method: clinit_method,
                },
                None,
                Vec::new(),
            )?;
        }
        Ok(())
    }

    pub fn get_class_by_id(&self, class_id: ClassId) -> Result<ClassRef<'a>, VmError> {
        self.find_class_by_id(class_id)
            .ok_or(VmError::ValidationException)
    }

    pub fn find_class_by_id(&self, class_id: ClassId) -> Option<ClassRef<'a>> {
        self.class_manager.find_class_by_id(class_id)
    }

    pub fn find_class_by_name(&self, class_name: &str) -> Option<ClassRef<'a>> {
        self.class_manager.find_class_by_name(class_name)
    }

    pub fn resolve_class_method(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_name: &str,
        method_name: &str,
        method_type_descriptor: &str,
    ) -> Result<ClassAndMethod<'a>, MethodCallFailed<'a>> {
        self.get_or_resolve_class(call_stack, class_name)
            .and_then(|class| {
                class
                    .find_method(method_name, method_type_descriptor)
                    .map(|method| ClassAndMethod { class, method })
                    .ok_or(MethodCallFailed::InternalError(
                        VmError::ClassNotFoundException(class_name.to_string()),
                    ))
            })
    }

    pub fn invoke(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_and_method: ClassAndMethod<'a>,
        object: Option<ObjectRef<'a>>,
        args: Vec<Value<'a>>,
    ) -> MethodCallResult<'a> {
        if class_and_method.method.is_native() {
            return self.invoke_native(call_stack, class_and_method, object, args);
        }

        let frame = call_stack.add_frame(class_and_method, object, args)?;
        let result = frame.borrow_mut().execute(self, call_stack);
        call_stack
            .pop_frame()
            .expect("should be able to pop the frame we just pushed");
        result
    }

    fn invoke_native(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_and_method: ClassAndMethod<'a>,
        object: Option<ObjectRef<'a>>,
        args: Vec<Value<'a>>,
    ) -> MethodCallResult<'a> {
        let native_callback = self.native_methods_registry.get_method(&class_and_method);
        if let Some(native_callback) = native_callback {
            debug!(
                "executing native method {}::{} {}",
                class_and_method.class.name,
                class_and_method.method.name,
                class_and_method.method.type_descriptor
            );
            native_callback(self, call_stack, object, args)
        } else {
            error!(
                "cannot resolve native method {}::{} {}",
                class_and_method.class.name,
                class_and_method.method.name,
                class_and_method.method.type_descriptor
            );
            Err(MethodCallFailed::InternalError(VmError::NotImplemented))
        }
    }

    // TODO: do we need it?
    pub fn allocate_call_stack(&self) -> CallStack<'a> {
        CallStack::new()
    }

    pub fn new_object(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_name: &str,
    ) -> Result<ObjectRef<'a>, MethodCallFailed<'a>> {
        let class = self.get_or_resolve_class(call_stack, class_name)?;
        Ok(self.new_object_of_class(class))
    }

    pub fn new_object_of_class(&mut self, class: ClassRef<'a>) -> ObjectRef<'a> {
        debug!("allocating new instance of {}", class.name);
        self.object_allocator.allocate(class)
    }

    pub fn create_java_lang_string_instance(
        &mut self,
        call_stack: &mut CallStack<'a>,
        string: &str,
    ) -> Result<ObjectRef<'a>, MethodCallFailed<'a>> {
        let char_array: Vec<Value<'a>> = string
            .encode_utf16()
            .map(|c| Value::Int(c as i32))
            .collect();
        let char_array = Rc::new(RefCell::new(char_array));
        let char_array = Value::Array(FieldType::Base(BaseType::Char), char_array);

        // In our JRE's rt.jar, the fields for String are:
        //    private final char[] value;
        //    private int hash;
        //    private static final long serialVersionUID = -6849794470754667710L;
        //    private static final ObjectStreamField[] serialPersistentFields = new ObjectStreamField[0];
        //    public static final Comparator<String> CASE_INSENSITIVE_ORDER = new CaseInsensitiveComparator();
        //    private static final int HASHING_SEED;
        //    private transient int hash32;
        let string_object = self.new_object(call_stack, "java/lang/String")?;
        string_object.set_field(0, char_array);
        string_object.set_field(1, Value::Int(0));
        string_object.set_field(6, Value::Int(0));
        Ok(string_object)
    }

    pub fn create_instance_of_java_lang_class(
        &mut self,
        call_stack: &mut CallStack<'a>,
        class_name: &str,
    ) -> Result<ObjectRef<'a>, MethodCallFailed<'a>> {
        let class_object = self.new_object(call_stack, "java/lang/Class")?;
        // TODO: build a proper instance of Class object
        let string_object = Self::create_java_lang_string_instance(self, call_stack, class_name)?;
        class_object.set_field(5, Value::Object(string_object));
        Ok(class_object)
    }

    pub fn debug_stats(&self) {
        debug!(
            "VM classes={:?}, objects = {:?}",
            self.class_manager, self.object_allocator
        )
    }
}
