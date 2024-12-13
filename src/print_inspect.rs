use std::io;

pub struct PrintInspect;

impl crate::Inspect for PrintInspect {
    fn read(&mut self, value: &io::Result<&[u8]>) {
        match value {
            Ok(buf) => println!("read: {}", buf.len()),
            Err(e) => eprintln!("{e}"),
        }
    }

    fn write(&mut self, value: &io::Result<&[u8]>) {
        match value {
            Ok(buf) => println!("write: {}", buf.len()),
            Err(e) => eprintln!("{e}"),
        }
    }

    fn flush(&mut self, value: &io::Result<()>) {
        match value {
            Ok(_) => println!("flush"),
            Err(e) => eprintln!("{e}"),
        }
    }

    fn shutdown(&mut self, value: &io::Result<()>) {
        match value {
            Ok(_) => println!("shutdown"),
            Err(e) => eprintln!("{e}"),
        }
    }

    fn write_vectored(&mut self, value: &io::Result<(&[io::IoSlice<'_>], usize)>) {
        match value {
            Ok((_, len)) => println!("write_vectored: {len}"),
            Err(e) => eprintln!("{e}"),
        }
    }
}

impl Drop for PrintInspect {
    fn drop(&mut self) {
        println!("drop");
    }
}
