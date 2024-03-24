use std::fmt::{Debug, Display, Write};
use bytesutil::WriteTo;
use crate::profiler::log_msg::Log;

#[derive(Copy, Clone)]
enum FieldType {
    I8 = 1,
    I16 = 2,
    I32 = 3,
    I64 = 4,
    U8 = 5,
    U16 = 6,
    U32 = 7,
    U64 = 8,
    F64 = 9,
    STR = 10,
    BOOL = 11
}

impl FieldValue for FieldType {
    fn write_field<W: Log>(&self, log: &mut W) -> std::io::Result<()> {
        unsafe { log.write_single(*self as u8) };
        Ok(())
    }
}

pub trait FieldValue {
    fn write_field<W: Log>(&self, log: &mut W) -> std::io::Result<()>;
}

impl FieldValue for u64 {
    fn write_field<W: Log>(&self, log: &mut W) -> std::io::Result<()> {
        if *self < 256 {
            FieldType::U8.write_field(log)?;
            unsafe { log.write_single(*self as u8) };
        } else if *self < 65536 {
            FieldType::U16.write_field(log)?;
            (*self as u16).write_to_le(log)?;
        } else if *self < u32::MAX as u64 + 1 {
            FieldType::U32.write_field(log)?;
            (*self as u32).write_to_le(log)?;
        } else {
            FieldType::U64.write_field(log)?;
            self.write_to_le(log)?;
        }
        Ok(())
    }
}

impl FieldValue for i64 {
    fn write_field<W: Log>(&self, log: &mut W) -> std::io::Result<()> {
        if *self < 128 {
            FieldType::I8.write_field(log)?;
            unsafe { log.write_single(*self as i8 as u8) };
        } else if *self < 32768 {
            FieldType::I16.write_field(log)?;
            (*self as i16).write_to_le(log)?;
        } else if *self < i32::MAX as i64 + 1 {
            FieldType::I32.write_field(log)?;
            (*self as i32).write_to_le(log)?;
        } else {
            FieldType::I64.write_field(log)?;
            self.write_to_le(log)?;
        }
        Ok(())
    }
}

impl<D: Display> FieldValue for &D {
    fn write_field<W: Log>(&self, log: &mut W) -> std::io::Result<()> {
        FieldType::STR.write_field(log)?;
        let mut formatter = Formatter::new(log);
        let _ = write!(formatter, "{}", self);
        Ok(())
    }
}

impl FieldValue for &dyn Debug {
    fn write_field<W: Log>(&self, log: &mut W) -> std::io::Result<()> {
        FieldType::STR.write_field(log)?;
        let mut formatter = Formatter::new(log);
        let _ = write!(formatter, "{:?}", self);
        Ok(())
    }
}

impl FieldValue for &str {
    fn write_field<W: Log>(&self, log: &mut W) -> std::io::Result<()> {
        FieldType::STR.write_field(log)?;
        let mut formatter = Formatter::new(log);
        let _ = formatter.write_str(self);
        Ok(())
    }
}

impl FieldValue for f64 {
    fn write_field<W: Log>(&self, log: &mut W) -> std::io::Result<()> {
        FieldType::F64.write_field(log)?;
        self.write_to_le(log)
    }
}

impl FieldValue for bool {
    fn write_field<W: Log>(&self, log: &mut W) -> std::io::Result<()> {
        FieldType::BOOL.write_field(log)?;
        match *self {
            true => unsafe { log.write_single(1) },
            false => unsafe { log.write_single(0) }
        }
        Ok(())
    }
}

pub struct Field<'a, V: FieldValue> {
    pub name: &'a str,
    pub value: V
}

impl<'a, V: FieldValue> Field<'a, V> {
    pub fn new(name: &'a str, value: V) -> Field<'a, V> {
        Field {
            name,
            value
        }
    }

    pub fn write_into<W: Log>(self, log: &mut W) {
        {
            //rust is too stupid to understand that formatter is not after write_str
            let mut formatter = Formatter::new(log);
            let _ = formatter.write_str(self.name);
        }
        let _ = self.value.write_field(log);
        log.increment_var_count();
    }
}

pub struct Formatter<'a, W: Log>(&'a mut W);

impl<'a, W: Log> Formatter<'a, W> {
    pub fn new(log: &'a mut W) -> Self {
        Self(log)
    }
}

impl<'a, W: Log> Write for Formatter<'a, W> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        unsafe {
            self.0.write_multiple(s.as_bytes());
        }
        Ok(())
    }
}

impl<'a, W: Log> Drop for Formatter<'a, W> {
    fn drop(&mut self) {
        unsafe {
            self.0.write_single(0);
        }
    }
}
