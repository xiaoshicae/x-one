//! 异步日志写入器
//!
//! 通过 channel + 后台线程将同步写入转为异步，
//! 避免日志 I/O 阻塞调用方。

use std::io::{self, Write};
use std::sync::Once;
use std::sync::mpsc;

/// 默认异步缓冲区大小
const DEFAULT_ASYNC_BUFFER_SIZE: usize = 4096;

/// 异步写入器
pub struct AsyncWriter {
    tx: Option<mpsc::SyncSender<Vec<u8>>>,
    join_handle: Option<std::thread::JoinHandle<()>>,
    close_once: Once,
}

impl AsyncWriter {
    /// 创建异步写入器
    ///
    /// `writer` 是底层写入目标，`buffer_size` 是 channel 缓冲区大小。
    pub fn new<W: Write + Send + 'static>(writer: W, buffer_size: usize) -> Self {
        let size = if buffer_size == 0 {
            DEFAULT_ASYNC_BUFFER_SIZE
        } else {
            buffer_size
        };

        let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(size);

        let join_handle = std::thread::spawn(move || {
            let mut writer = writer;
            while let Ok(buf) = rx.recv() {
                let _ = writer.write_all(&buf);
            }
            let _ = writer.flush();
        });

        Self {
            tx: Some(tx),
            join_handle: Some(join_handle),
            close_once: Once::new(),
        }
    }
}

impl Write for AsyncWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(tx) = &self.tx {
            // 必须拷贝，因为调用方可能复用 buffer
            let data = buf.to_vec();
            let len = data.len();
            tx.send(data)
                .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e.to_string()))?;
            Ok(len)
        } else {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "writer closed"))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsyncWriter {
    /// 关闭异步写入器，等待所有数据写完
    pub fn close(&mut self) {
        self.close_once.call_once(|| {
            // 丢弃 sender，通知接收端关闭
            self.tx.take();
        });
        // 等待后台线程完成
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for AsyncWriter {
    fn drop(&mut self) {
        self.close();
    }
}
