use std::ops::Index;
use std::slice::{Iter, SliceIndex};

use VmError::ValidationException;

use crate::value::Value;
use crate::vm_error::VmError;

#[derive(Debug)]
pub struct ValueStack<'a> {
    stack: Vec<Value<'a>>,
}

impl<'a> ValueStack<'a> {
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            stack: Vec::with_capacity(max_size),
        }
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn push(&mut self, value: Value<'a>) -> Result<(), VmError> {
        if self.stack.len() < self.stack.capacity() {
            self.stack.push(value);
            Ok(())
        } else {
            Err(ValidationException)
        }
    }

    pub fn pop(&mut self) -> Result<Value<'a>, VmError> {
        self.stack.pop().ok_or(ValidationException)
    }

    pub fn pop2(&mut self) -> Result<Value<'a>, VmError> {
        let value = self.pop()?;
        match value {
            Value::Long(_) | Value::Double(_) => Ok(value),
            _ => self.pop().map(|_| value),
        }
    }

    pub fn truncate(&mut self, len: usize) -> Result<(), VmError> {
        if len > self.stack.capacity() {
            Err(ValidationException)
        } else {
            self.stack.truncate(len);
            Ok(())
        }
    }

    pub fn get(&self, index: usize) -> Option<&Value<'a>> {
        self.stack.get(index)
    }

    pub fn iter(&self) -> Iter<Value<'a>> {
        self.stack.iter()
    }

    pub fn dup(&mut self) -> Result<(), VmError> {
        if self.stack.len() < self.stack.capacity() {
            match self.stack.last() {
                None => Err(ValidationException),
                Some(head) => {
                    self.stack.push(head.clone());
                    Ok(())
                }
            }
        } else {
            Err(ValidationException)
        }
    }

    pub fn dup_x1(&mut self) -> Result<(), VmError> {
        if self.stack.len() < self.stack.capacity() {
            let value1 = self.pop()?;
            let value2 = self.pop()?;
            self.push(value1.clone())?;
            self.push(value2)?;
            self.push(value1)
        } else {
            Err(ValidationException)
        }
    }

    pub fn dup_x2(&mut self) -> Result<(), VmError> {
        if self.stack.len() < self.stack.capacity() {
            let value1 = self.pop()?;
            let value2 = self.pop()?;
            let value3 = self.pop()?;
            self.push(value1.clone())?;
            self.push(value3)?;
            self.push(value2)?;
            self.push(value1)
        } else {
            Err(ValidationException)
        }
    }

    pub fn dup2(&mut self) -> Result<(), VmError> {
        if self.stack.len() < self.stack.capacity() {
            let value1 = self.pop()?;
            let value2 = self.pop()?;
            self.push(value2.clone())?;
            self.push(value1.clone())?;
            self.push(value2)?;
            self.push(value1)
        } else {
            Err(ValidationException)
        }
    }

    pub fn dup2_x1(&mut self) -> Result<(), VmError> {
        if self.stack.len() < self.stack.capacity() {
            let value1 = self.pop()?;
            let value2 = self.pop()?;
            let value3 = self.pop()?;
            self.push(value2.clone())?;
            self.push(value1.clone())?;
            self.push(value3)?;
            self.push(value2)?;
            self.push(value1)
        } else {
            Err(ValidationException)
        }
    }

    pub fn dup2_x2(&mut self) -> Result<(), VmError> {
        if self.stack.len() < self.stack.capacity() {
            let value1 = self.pop()?;
            let value2 = self.pop()?;
            let value3 = self.pop()?;
            let value4 = self.pop()?;
            self.push(value2.clone())?;
            self.push(value1.clone())?;
            self.push(value4)?;
            self.push(value3)?;
            self.push(value2)?;
            self.push(value1)
        } else {
            Err(ValidationException)
        }
    }

    pub fn swap(&mut self) -> Result<(), VmError> {
        let value1 = self.pop()?;
        let value2 = self.pop()?;
        self.push(value1)?;
        self.push(value2)
    }
}

impl<'a, I> Index<I> for ValueStack<'a>
where
    I: SliceIndex<[Value<'a>]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.stack.index(index)
    }
}

#[cfg(test)]
mod tests {
    use crate::value::Value;
    use crate::value_stack::ValueStack;

    #[test]
    fn can_do_push_pop_and_indexing() {
        let mut stack = ValueStack::with_max_size(3);
        stack.push(Value::Int(1)).expect("should be able to push");
        stack.push(Value::Int(2)).expect("should be able to push");
        stack.push(Value::Int(3)).expect("should be able to push");

        assert_eq!(Ok(Value::Int(3)), stack.pop());
        assert_eq!(Some(&Value::Int(1)), stack.get(0));
        assert_eq!(Value::Int(2), stack[1]);
        assert_eq!(2, stack.len());

        stack.truncate(1).expect("should be able to truncate");
        assert_eq!(1, stack.len());
        assert_eq!(Ok(Value::Int(1)), stack.pop());
    }

    #[test]
    fn cannot_push_above_capacity() {
        let mut stack = ValueStack::with_max_size(1);
        stack.push(Value::Int(1)).expect("should be able to push");
        assert!(stack.push(Value::Int(2)).is_err());
    }

    #[test]
    fn can_invoke_dup() {
        let mut stack = ValueStack::with_max_size(2);
        stack.push(Value::Int(1)).expect("should be able to push");
        stack.dup().expect("should be able to dup");
        assert_eq!(2, stack.len());
        assert_eq!(Ok(Value::Int(1)), stack.pop());
        assert_eq!(Ok(Value::Int(1)), stack.pop());
    }

    #[test]
    fn can_invoke_dup_x1() {
        let mut stack = ValueStack::with_max_size(3);
        stack.push(Value::Int(2)).expect("should be able to push");
        stack.push(Value::Int(1)).expect("should be able to push");
        stack.dup_x1().expect("should be able to dup_x1");
        assert_eq!(3, stack.len());
        assert_eq!(Ok(Value::Int(1)), stack.pop());
        assert_eq!(Ok(Value::Int(2)), stack.pop());
        assert_eq!(Ok(Value::Int(1)), stack.pop());
    }

    #[test]
    fn can_invoke_dup_x2() {
        let mut stack = ValueStack::with_max_size(4);
        stack.push(Value::Int(3)).expect("should be able to push");
        stack.push(Value::Int(2)).expect("should be able to push");
        stack.push(Value::Int(1)).expect("should be able to push");
        stack.dup_x2().expect("should be able to dup_x2");
        assert_eq!(4, stack.len());
        assert_eq!(Ok(Value::Int(1)), stack.pop());
        assert_eq!(Ok(Value::Int(2)), stack.pop());
        assert_eq!(Ok(Value::Int(3)), stack.pop());
        assert_eq!(Ok(Value::Int(1)), stack.pop());
    }

    #[test]
    fn can_invoke_dup2() {
        let mut stack = ValueStack::with_max_size(4);
        stack.push(Value::Int(2)).expect("should be able to push");
        stack.push(Value::Int(1)).expect("should be able to push");
        stack.dup2().expect("should be able to dup2");
        assert_eq!(4, stack.len());
        assert_eq!(Ok(Value::Int(1)), stack.pop());
        assert_eq!(Ok(Value::Int(2)), stack.pop());
        assert_eq!(Ok(Value::Int(1)), stack.pop());
        assert_eq!(Ok(Value::Int(2)), stack.pop());
    }

    #[test]
    fn can_invoke_dup2_x1() {
        let mut stack = ValueStack::with_max_size(5);
        stack.push(Value::Int(3)).expect("should be able to push");
        stack.push(Value::Int(2)).expect("should be able to push");
        stack.push(Value::Int(1)).expect("should be able to push");
        stack.dup2_x1().expect("should be able to dup2_x1");
        assert_eq!(5, stack.len());
        assert_eq!(Ok(Value::Int(1)), stack.pop());
        assert_eq!(Ok(Value::Int(2)), stack.pop());
        assert_eq!(Ok(Value::Int(3)), stack.pop());
        assert_eq!(Ok(Value::Int(1)), stack.pop());
        assert_eq!(Ok(Value::Int(2)), stack.pop());
    }

    #[test]
    fn can_invoke_dup2_x2() {
        let mut stack = ValueStack::with_max_size(6);
        stack.push(Value::Int(4)).expect("should be able to push");
        stack.push(Value::Int(3)).expect("should be able to push");
        stack.push(Value::Int(2)).expect("should be able to push");
        stack.push(Value::Int(1)).expect("should be able to push");
        stack.dup2_x2().expect("should be able to dup2_x2");
        assert_eq!(6, stack.len());
        assert_eq!(Ok(Value::Int(1)), stack.pop());
        assert_eq!(Ok(Value::Int(2)), stack.pop());
        assert_eq!(Ok(Value::Int(3)), stack.pop());
        assert_eq!(Ok(Value::Int(4)), stack.pop());
        assert_eq!(Ok(Value::Int(1)), stack.pop());
        assert_eq!(Ok(Value::Int(2)), stack.pop());
    }

    #[test]
    fn can_invoke_pop2() {
        let mut stack = ValueStack::with_max_size(4);
        stack
            .push(Value::Double(0f64))
            .expect("should be able to push");
        stack.push(Value::Int(1)).expect("should be able to push");
        stack.push(Value::Int(2)).expect("should be able to push");
        stack.push(Value::Long(3)).expect("should be able to push");
        assert_eq!(Ok(Value::Long(3)), stack.pop2());
        assert_eq!(3, stack.len());
        assert_eq!(Ok(Value::Int(2)), stack.pop2());
        assert_eq!(1, stack.len());
        assert_eq!(Ok(Value::Double(0f64)), stack.pop2());
    }

    #[test]
    fn can_invoke_swap() {
        let mut stack = ValueStack::with_max_size(2);
        stack.push(Value::Int(1)).expect("should be able to push");
        stack.push(Value::Int(2)).expect("should be able to push");
        stack.swap().expect("should be able to swap");
        assert_eq!(2, stack.len());
        assert_eq!(Ok(Value::Int(1)), stack.pop());
        assert_eq!(Ok(Value::Int(2)), stack.pop());
    }
}
