/**
 * This transform validates the content of the record. If the content is valid JSON, it will be written to the "verified" topic.
 * If the content is not valid JSON, it will be written back to the "unprocessed" topic with an incremented attempt count.
 * If the attempt count is greater than 3, the record will be written to the "unprocessable" topic.
 */
use redpanda_transform_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::error::Error;

#[derive(Serialize, Deserialize)]
struct LLMResult {
    attempt: i32,
    content: String,
}

const SCHEMA_ID: i32 = 2;
const MAX_ATTEMPTS: i32 = 3;

fn main() {
    on_record_written(my_transform);
}

fn my_transform(event: WriteEvent, writer: &mut RecordWriter) -> Result<(), Box<dyn Error>> {
    // Deserialize the input record
    let mut input_record: LLMResult = serde_json::from_slice(event.record.value().unwrap())
        .expect("Failed to deserialize input record");

    // Attempt to parse the content as JSON
    if let Ok(_parsed_json) = serde_json::from_str::<Value>(&input_record.content) {
        // This is valid JSON, but we need to check if it is a valid schema

        // Get the schema from the schema registry
        let client = redpanda_transform_sdk_sr::SchemaRegistryClient::new();
        let schema = client.lookup_schema_by_id(redpanda_transform_sdk_sr::SchemaId(SCHEMA_ID)).expect("Failed to lookup schema");

        // Validate the record against the schema
        if jsonschema::is_valid(&json!(schema.schema()), &json!(event.record.value().unwrap())) {
            // Valid! Let's write it to the structured topic
            writer.write_with_options(event.record, WriteOptions::to_topic("structured"))?;
            return Ok(())
        }
    }

    // otherwise something has gone wrong

    // Increment the attempt count, serialize, and write back to try again
    input_record.attempt += 1;

    // If the attempt is greater than 3, write to "unprocessable" topic
    if input_record.attempt > MAX_ATTEMPTS {
        writer.write_with_options(event.record, WriteOptions::to_topic("unprocessable"))?;
        return Ok(());
    }

    let new_record =
        serde_json::to_string(&input_record).expect("Failed to serialize new record");
    writer.write_with_options(
        BorrowedRecord::new(event.record.key(), Some(&new_record.as_bytes())),
        WriteOptions::to_topic("unprocessed"),
    )?;
    eprintln!("Invalid JSON in content field");
    writer.write_with_options(event.record, WriteOptions::to_topic("unprocessable"))?;
    Ok(())
}
