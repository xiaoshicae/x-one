use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use x_one::xlog::async_writer::*;

/// 用于测试的内存写入器
struct MemWriter {
    data: Arc<Mutex<Vec<u8>>>,
}

impl Write for MemWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.data.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_async_writer_write_and_close() {
    let data = Arc::new(Mutex::new(Vec::new()));
    let writer = MemWriter { data: data.clone() };

    let mut aw = AsyncWriter::new(writer, 100);
    aw.write_all(b"hello ").unwrap();
    aw.write_all(b"world").unwrap();
    aw.close();

    assert_eq!(&*data.lock().unwrap(), b"hello world");
}

#[test]
fn test_async_writer_multiple_writes() {
    let data = Arc::new(Mutex::new(Vec::new()));
    let writer = MemWriter { data: data.clone() };

    let mut aw = AsyncWriter::new(writer, 100);
    for i in 0..10 {
        aw.write_all(format!("{i}").as_bytes()).unwrap();
    }
    aw.close();

    let result = String::from_utf8(data.lock().unwrap().clone()).unwrap();
    assert_eq!(result, "0123456789");
}

#[test]
fn test_async_writer_drop_closes() {
    let data = Arc::new(Mutex::new(Vec::new()));
    let writer = MemWriter { data: data.clone() };

    {
        let mut aw = AsyncWriter::new(writer, 100);
        aw.write_all(b"dropped").unwrap();
        // drop 时自动关闭
    }

    assert_eq!(&*data.lock().unwrap(), b"dropped");
}

#[test]
fn test_async_writer_default_buffer_size() {
    let data = Arc::new(Mutex::new(Vec::new()));
    let writer = MemWriter { data: data.clone() };

    let mut aw = AsyncWriter::new(writer, 0);
    aw.write_all(b"test").unwrap();
    aw.close();

    assert_eq!(&*data.lock().unwrap(), b"test");
}
