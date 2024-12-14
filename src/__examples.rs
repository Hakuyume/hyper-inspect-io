use std::io;

pub struct PrintInspect;

impl crate::InspectRead for PrintInspect {
    fn inspect_read(&mut self, value: Result<&[u8], &io::Error>) {
        match value {
            Ok(buf) => println!("read: {}", buf.len()),
            Err(e) => eprintln!("read: {e}"),
        }
    }
}

impl crate::InspectWrite for PrintInspect {
    fn inspect_write(&mut self, value: Result<&[u8], &io::Error>) {
        match value {
            Ok(buf) => println!("write: {}", buf.len()),
            Err(e) => eprintln!("write: {e}"),
        }
    }

    fn inspect_flush(&mut self, value: Result<(), &io::Error>) {
        match value {
            Ok(_) => println!("flush"),
            Err(e) => eprintln!("flush: {e}"),
        }
    }

    fn inspect_shutdown(&mut self, value: Result<(), &io::Error>) {
        match value {
            Ok(_) => println!("shutdown"),
            Err(e) => eprintln!("shutdown: {e}"),
        }
    }

    fn inspect_write_vectored<'a, I>(&mut self, value: Result<I, &io::Error>)
    where
        I: Iterator<Item = &'a [u8]>,
    {
        match value {
            Ok(bufs) => println!("write_vectored: {}", bufs.map(<[_]>::len).sum::<usize>()),
            Err(e) => eprintln!("write_vectored: {e}"),
        }
    }
}

impl Drop for PrintInspect {
    fn drop(&mut self) {
        println!("drop");
    }
}
