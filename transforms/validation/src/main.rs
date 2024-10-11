/**
 * This transform validates the content of the record. If the content is valid JSON, it will be written to the "verified" topic.
 * If the content is not valid JSON, it will be written back to the "unprocessed" topic with an incremented attempt count.
 * If the attempt count is greater than 3, the record will be written to the "unprocessable" topic.
 */
use redpanda_transform_sdk::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::error::Error;

#[derive(Serialize, Deserialize)]
struct LLMResult {
    attempt: i32,
    // Original email content
    content: String,
    // Output from the LLM
    output: Value,
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

    // Get the schema from the schema registry
    let client = redpanda_transform_sdk_sr::SchemaRegistryClient::new();
    let schema = client
        .lookup_schema_by_id(redpanda_transform_sdk_sr::SchemaId(SCHEMA_ID))
        .expect("Failed to lookup schema");

    // Attempt to parse the content as JSON
    match input_record.output {
        Value::String(ref str) => {
            // A string, let's check if it's valid json
            if let Ok(_parsed_json) = serde_json::from_str::<Value>(str) {
                // This is valid JSON, but we need to check if it is a valid schema
                if jsonschema::is_valid(
                    &json!(schema.schema()),
                    &json!(_parsed_json),
                ) {
                    // Valid! Let's write it to the structured topic
                    writer
                        .write_with_options(event.record, WriteOptions::to_topic("structured"))?;
                    return Ok(());
                } else {
                    eprintln!("output did not match schema");
                }
            } else {
                eprintln!("Invalid JSON string in content field");
            }
            // otherwise we'll fail-through
        },
        Value::Object(_) => {
            // This is valid JSON, but we need to check if it is a valid schema
            if jsonschema::is_valid(
                &json!(schema.schema()),
                &input_record.output,
            ) {
                // Valid! Let's write it to the structured topic
                writer
                    .write_with_options(event.record, WriteOptions::to_topic("structured"))?;
                return Ok(());
            }
            eprintln!("output object did not match schema");
        },
        _ => {
            return record_failed(event, &mut input_record, writer);
        }
    }

    // we failed something
    record_failed(event, &mut input_record, writer)
}

fn record_failed(
    event: WriteEvent,
    input_record: &mut LLMResult,
    writer: &mut RecordWriter,
) -> Result<(), Box<dyn Error>> {
    // Increment the attempt count, serialize, and write back to try again
    input_record.attempt += 1;

    // If the attempt is greater than 3, write to "unprocessable" topic
    if input_record.attempt > MAX_ATTEMPTS {
        writer.write_with_options(event.record, WriteOptions::to_topic("unprocessable"))?;
        eprintln!("Record is unprocessable");
        return Ok(());
    }

    let new_record = serde_json::to_string(&input_record).expect("Failed to serialize new record");
    writer.write_with_options(
        BorrowedRecord::new(event.record.key(), Some(&new_record.as_bytes())),
        WriteOptions::to_topic("unprocessed"),
    )?;
    eprintln!("Invalid JSON in content field");
    Ok(())
}
