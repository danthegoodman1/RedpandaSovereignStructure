/**
 * This transform validates the content of the record. If the content is valid JSON, it will be written to the "verified" topic.
 * If the content is not valid JSON, it will be written back to the "unprocessed" topic with an incremented attempt count.
 * If the attempt count is greater than 3, the record will be written to the "unprocessable" topic.
 */

use base64::prelude::*;
use redpanda_transform_sdk::*;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize, Deserialize)]
struct InputRecord {
    attempt: i32,
    // Base64 encoded string
    content: String,
}

fn main() {
    // Register your transform function.
    // This is a good place to perform other setup too.
    on_record_written(my_transform);
}

// my_transform is where you read the record that was written, and then you can
// return new records that will be written to the output topic
fn my_transform(event: WriteEvent, writer: &mut RecordWriter) -> Result<(), Box<dyn Error>> {
    // Deserialize the input record
    let mut input_record: InputRecord = serde_json::from_slice(event.record.value().unwrap())
        .expect("Failed to deserialize input record");

    // If the attempt is greater than 3, write to "unprocessable" topic
    if input_record.attempt > 3 {
        writer.write_with_options(event.record, WriteOptions::to_topic("unprocessable"))?;
        return Ok(());
    }

    // Decode the base64 content
    match BASE64_STANDARD.decode(&input_record.content) {
        Ok(decoded_content) => {
            // Check if the decoded content is valid JSON
            if let Ok(_) = serde_json::from_slice::<serde_json::Value>(&decoded_content) {
                // This is valid JSON, write to verified topic
                writer.write_with_options(event.record, WriteOptions::to_topic("verified"))?;
                return Ok(());
            } else {
                // Increment the attempt count, serialize, and write back to try again
                input_record.attempt += 1;
                let new_record =
                    serde_json::to_string(&input_record).expect("Failed to serialize new record");
                writer.write_with_options(
                    BorrowedRecord::new(event.record.key(), Some(&new_record.as_bytes())),
                    WriteOptions::to_topic("unprocessed"),
                )?;
                return Err("Invalid JSON in content field".into());
            }
        }
        Err(_) => {
            return Err("Invalid base64 encoding in content field".into());
        }
    }

    // Existing code to write the record
    writer.write(event.record)?;
    Ok(())
}
