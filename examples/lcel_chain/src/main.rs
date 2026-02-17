use serde_json::{json, Value};
use synaptic::core::{RunnableConfig, SynapseError};
use synaptic::runnables::{
    BoxRunnable, Runnable, RunnableLambda, RunnableParallel, RunnablePassthrough,
};

#[tokio::main]
async fn main() -> Result<(), SynapseError> {
    let config = RunnableConfig::default();

    // --- RunnableLambda ---
    println!("=== RunnableLambda ===");
    let upper = RunnableLambda::new(|s: String| async move { Ok(s.to_uppercase()) });
    let result = upper.invoke("hello synapse".to_string(), &config).await?;
    println!("upper: {result}");

    // --- Pipe operator ---
    println!("\n=== Pipe Operator ===");
    let exclaim = RunnableLambda::new(|s: String| async move { Ok(format!("{s}!")) });
    let chain = upper.boxed() | exclaim.boxed();
    let result = chain.invoke("hello".to_string(), &config).await?;
    println!("upper | exclaim: {result}");

    // --- Three-step pipeline ---
    println!("\n=== Three-Step Pipeline ===");
    let step1 = RunnableLambda::new(|s: String| async move { Ok(s.trim().to_string()) });
    let step2 = RunnableLambda::new(|s: String| async move { Ok(s.to_uppercase()) });
    let step3 = RunnableLambda::new(|s: String| async move { Ok(format!("[{s}]")) });
    let pipeline = step1.boxed() | step2.boxed() | step3.boxed();
    let result = pipeline
        .invoke("  hello world  ".to_string(), &config)
        .await?;
    println!("trim | upper | bracket: {result}");

    // --- RunnableParallel ---
    println!("\n=== RunnableParallel ===");
    let branches = RunnableParallel::new(vec![
        (
            "upper".to_string(),
            RunnableLambda::new(|s: String| async move { Ok(Value::String(s.to_uppercase())) })
                .boxed(),
        ),
        (
            "lower".to_string(),
            RunnableLambda::new(|s: String| async move { Ok(Value::String(s.to_lowercase())) })
                .boxed(),
        ),
        (
            "length".to_string(),
            RunnableLambda::new(|s: String| async move { Ok(json!(s.len())) }).boxed(),
        ),
    ]);
    let result = branches.invoke("Hello World".to_string(), &config).await?;
    println!("parallel: {result}");

    // --- RunnablePassthrough ---
    println!("\n=== RunnablePassthrough ===");
    let passthrough: BoxRunnable<String, String> = RunnablePassthrough.boxed();
    let result = passthrough.invoke("unchanged".to_string(), &config).await?;
    println!("passthrough: {result}");

    println!("\nAll LCEL demos completed successfully!");
    Ok(())
}
