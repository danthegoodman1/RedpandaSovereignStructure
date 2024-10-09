/**
 * This transform just creates a wrapper for counting attempts so that we can eventually put this record in the dead letter queue if needed
 */

use redpanda_transform_sdk::*;
use std::error::Error;
use serde::Serialize;
use base64::prelude::*;

#[derive(Serialize)]
struct OutputRecord {
    attempts: i32,
    original: String,
}

fn main() {
    on_record_written(my_transform);
}

// my_transform is where you read the record that was written, and then you can
// return new records that will be written to the output topic
fn my_transform(event: WriteEvent, writer: &mut RecordWriter) -> Result<(), Box<dyn Error>> {
    let output_record = OutputRecord {
        attempts: 0,
        original: BASE64_STANDARD.encode(event.record.value().expect("Failed to get record value")),
    };
    
    let json_string = serde_json::to_string(&output_record)?;
    writer.write(BorrowedRecord::new(event.record.key(), Some(&json_string.as_bytes())))?;
    Ok(())
}
