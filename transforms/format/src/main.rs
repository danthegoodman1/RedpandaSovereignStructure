/**
 * This transform just creates a wrapper for counting attempts so that we can eventually put this record in the dead letter queue if needed
 */

use redpanda_transform_sdk::*;
use serde_json::Value;
use std::{error::Error};
use serde::Serialize;

#[derive(Serialize)]
struct RecordAttempted {
    attempts: i32,
    content: Value,
}

const SCHEMA_ID: i32 = 1;

fn main() {
    on_record_written(my_transform);
}

// my_transform is where you read the record that was written, and then you can
// return new records that will be written to the output topic
fn my_transform(event: WriteEvent, writer: &mut RecordWriter) -> Result<(), Box<dyn Error>> {
    let content = if let Some(value) = event.record.value() {
        // Parse the JSON-encoded bytes into a serde_json::Value
        match serde_json::from_slice(value) {
            Ok(value) => value,
            Err(e) => {
                // If not JSON, return string
                Value::String(String::from_utf8(value.to_vec())?)
            }
        }
    } else {
        Value::Null // Use Value::Null if no value is present
    };

    let output_record = RecordAttempted {
        attempts: 0,
        content: content,
    };

    let encoded_record = redpanda_transform_sdk_sr::encode_schema_id(
        redpanda_transform_sdk_sr::SchemaId(SCHEMA_ID),
        &serde_json::to_string(&output_record)?.as_bytes()
    );
    writer.write(BorrowedRecord::new(event.record.key(), Some(&encoded_record)))?;
    Ok(())
}
