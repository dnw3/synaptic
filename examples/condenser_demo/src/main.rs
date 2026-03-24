use synaptic::condenser::{CondenseContext, Condenser, NoOpCondenser};
use synaptic::core::Message;

#[tokio::main]
async fn main() {
    // Build a conversation with many messages
    let messages = vec![
        Message::human("What is Rust?"),
        Message::ai("Rust is a systems programming language focused on safety and performance."),
        Message::human("How does ownership work?"),
        Message::ai("Ownership is Rust's memory management model with three rules: each value has an owner, only one owner at a time, and the value is dropped when the owner goes out of scope."),
        Message::human("What about borrowing?"),
        Message::ai("Borrowing lets you reference data without taking ownership. You can have many immutable references or one mutable reference at a time."),
        Message::human("Can you explain lifetimes?"),
        Message::ai("Lifetimes are annotations that tell the compiler how long references are valid, preventing dangling references."),
        Message::human("What are traits?"),
        Message::ai("Traits define shared behavior, similar to interfaces in other languages."),
    ];

    println!("=== Condenser Demo ===\n");
    println!("Original message count: {}\n", messages.len());

    // NoOpCondenser: demonstrates the new CondenseContext API
    let condenser = NoOpCondenser;
    let ctx = CondenseContext {
        messages: messages.clone(),
        system_prompt: "You are a helpful assistant.".to_string(),
        tools: vec![],
        context_window: 128_000,
        reserved_output_tokens: 4096,
        has_thinking: false,
    };

    println!("Message budget: {} tokens", ctx.message_budget());
    println!(
        "Estimated message tokens: {} tokens",
        ctx.estimate_message_tokens()
    );

    let result = condenser.condense(ctx).await.unwrap();
    println!(
        "\n--- NoOpCondenser ---\nAction: {:?}\nMessages: {}\nEstimated tokens: {}",
        result.action,
        result.messages.len(),
        result.estimated_tokens
    );

    for msg in &result.messages {
        println!(
            "  [{}] {}...",
            msg.role(),
            &msg.content()[..msg.content().len().min(60)]
        );
    }

    println!("\nDone.");
}
