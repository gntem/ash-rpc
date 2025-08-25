
use ash_rpc_core::{MethodRegistry, MethodInfo, rpc_success};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("=== Documentation Caching Demo ===");
    
    let mut registry = MethodRegistry::new()
        .register_with_info(
            "test_method",
            MethodInfo::new("test_method")
                .with_description("A test method")
                .with_params_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Test message"
                        }
                    },
                    "required": ["message"]
                })),
            |_params, id| rpc_success!("test response", id)
        );

    println!("Registry created with {} methods", registry.method_count());
    
    println!("\n--- First render_docs() call (should generate docs) ---");
    let start = Instant::now();
    let docs1 = registry.render_docs();
    let first_duration = start.elapsed();
    println!("First call took: {:?}", first_duration);
    println!("Generated {} characters", serde_json::to_string(&docs1)?.len());
    
    println!("\n--- Second render_docs() call (should use cache) ---");
    let start = Instant::now();
    let docs2 = registry.render_docs();
    let second_duration = start.elapsed();
    println!("Second call took: {:?}", second_duration);
    println!("Retrieved {} characters", serde_json::to_string(&docs2)?.len());
    
    println!("Docs are identical: {}", 
        serde_json::to_string(&docs1)? == serde_json::to_string(&docs2)?);
    
    println!("\n--- Adding new method (should invalidate cache) ---");
    registry = registry.register("new_method", |_params, id| rpc_success!("new response", id));
    
    println!("\n--- Third render_docs() call (should regenerate) ---");
    let start = Instant::now();
    let docs3 = registry.render_docs();
    let third_duration = start.elapsed();
    println!("Third call took: {:?}", third_duration);
    println!("Generated {} characters", serde_json::to_string(&docs3)?.len());
    
    println!("\n=== Performance Summary ===");
    println!("First call (generate):  {:?}", first_duration);
    println!("Second call (cached):   {:?}", second_duration);
    println!("Third call (regenerate): {:?}", third_duration);
    
    let speedup = first_duration.as_nanos() as f64 / second_duration.as_nanos() as f64;
    println!("Cache speedup: {:.1}x faster", speedup);
    
    Ok(())
}
