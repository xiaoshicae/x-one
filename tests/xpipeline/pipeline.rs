use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;
use x_one::XOneError;
use x_one::xpipeline::{Frame, Pipeline, Processor};

/// 测试用文本帧
struct TextFrame(String);
impl Frame for TextFrame {
    fn frame_type(&self) -> &str {
        "text"
    }
}

/// 透传处理器
struct PassthroughProcessor {
    name: String,
}

impl Processor for PassthroughProcessor {
    fn name(&self) -> &str {
        &self.name
    }

    fn process<'a>(
        &'a self,
        input: &'a mut mpsc::Receiver<Box<dyn Frame>>,
        output: &'a mpsc::Sender<Box<dyn Frame>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), XOneError>> + Send + 'a>> {
        Box::pin(async move {
            while let Some(frame) = input.recv().await {
                let _ = output.send(frame).await;
            }
            Ok(())
        })
    }
}

/// 添加前缀处理器
struct PrefixProcessor;

impl Processor for PrefixProcessor {
    fn name(&self) -> &str {
        "prefixer"
    }

    fn process<'a>(
        &'a self,
        input: &'a mut mpsc::Receiver<Box<dyn Frame>>,
        output: &'a mpsc::Sender<Box<dyn Frame>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), XOneError>> + Send + 'a>> {
        Box::pin(async move {
            while let Some(frame) = input.recv().await {
                let _ = output.send(frame).await;
            }
            Ok(())
        })
    }
}

#[tokio::test]
async fn test_empty_pipeline() {
    let pipeline = Pipeline::new("empty");
    let (tx, mut rx, handle) = pipeline.run();

    // 关闭输入 → 转发 task 结束 → 输出 channel 关闭
    drop(tx);

    let result = handle.await.unwrap();
    assert!(result.success());

    // output channel 应已关闭
    assert!(rx.recv().await.is_none());
}

#[tokio::test]
async fn test_single_processor_passthrough() {
    let pipeline = Pipeline::new("passthrough").processor(PassthroughProcessor {
        name: "pass".to_string(),
    });

    let (tx, mut rx, handle) = pipeline.run();

    // 发送帧
    tx.send(Box::new(TextFrame("hello".to_string())))
        .await
        .unwrap();
    tx.send(Box::new(TextFrame("world".to_string())))
        .await
        .unwrap();
    drop(tx);

    // 接收帧
    let mut frames = Vec::new();
    while let Some(f) = rx.recv().await {
        frames.push(f);
    }

    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].frame_type(), "text");
    assert_eq!(frames[1].frame_type(), "text");

    let result = handle.await.unwrap();
    assert!(result.success());
}

#[tokio::test]
async fn test_multi_processor_chain() {
    let pipeline = Pipeline::new("chain")
        .processor(PassthroughProcessor {
            name: "p1".to_string(),
        })
        .processor(PassthroughProcessor {
            name: "p2".to_string(),
        });

    let (tx, mut rx, handle) = pipeline.run();

    tx.send(Box::new(TextFrame("hello".to_string())))
        .await
        .unwrap();
    drop(tx);

    let mut frames = Vec::new();
    while let Some(f) = rx.recv().await {
        frames.push(f);
    }

    assert_eq!(frames.len(), 1);

    let result = handle.await.unwrap();
    assert!(result.success());
}

/// 返回错误的处理器
struct ErrorProcessor;

impl Processor for ErrorProcessor {
    fn name(&self) -> &str {
        "error-proc"
    }

    fn process<'a>(
        &'a self,
        input: &'a mut mpsc::Receiver<Box<dyn Frame>>,
        _output: &'a mpsc::Sender<Box<dyn Frame>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), XOneError>> + Send + 'a>> {
        Box::pin(async move {
            // 消费所有输入
            while input.recv().await.is_some() {}
            Err(XOneError::Other("process failed".to_string()))
        })
    }
}

#[tokio::test]
async fn test_processor_error_recorded() {
    let pipeline = Pipeline::new("error-test").processor(ErrorProcessor);

    let (tx, mut rx, handle) = pipeline.run();
    drop(tx);

    // output channel 应正常关闭
    while rx.recv().await.is_some() {}

    let result = handle.await.unwrap();
    assert!(!result.success());
    assert!(result.has_errors());
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].processor_name, "error-proc");
}

#[tokio::test]
async fn test_pipeline_with_monitor() {
    use x_one::xpipeline::PipelineConfig;

    let pipeline = Pipeline::new("monitored")
        .config(PipelineConfig {
            buffer_size: 16,
            disable_monitor: false,
        })
        .enable_monitor()
        .processor(PassthroughProcessor {
            name: "p1".to_string(),
        });

    let (tx, mut rx, handle) = pipeline.run();
    tx.send(Box::new(TextFrame("test".to_string())))
        .await
        .unwrap();
    drop(tx);

    while rx.recv().await.is_some() {}

    let result = handle.await.unwrap();
    assert!(result.success());
}
