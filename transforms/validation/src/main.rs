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
        let (schema_id, record) = redpanda_transform_sdk_sr::decode_schema_id(&input_record.content.as_bytes())?;

        // Get the schema from the schema registry
        let client = redpanda_transform_sdk_sr::SchemaRegistryClient::new();
        let schema = client.lookup_schema_by_id(schema_id).expect("Failed to lookup schema");

        // Validate the record against the schema
        if jsonschema::is_valid(&json!(schema.schema()), &json!(record)) {
            // Valid! Let's write it to the verified topic
            writer.write_with_options(event.record, WriteOptions::to_topic("verified"))?;
            return Ok(())
        }
    }

    // otherwise something has gone wrong

    // Increment the attempt count, serialize, and write back to try again
    input_record.attempt += 1;

    // If the attempt is greater than 3, write to "unprocessable" topic
    if input_record.attempt > 3 {
        writer.write_with_options(event.record, WriteOptions::to_topic("unprocessable"))?;
        return Ok(());
    }

    let new_record =
        serde_json::to_string(&input_record).expect("Failed to serialize new record");
    writer.write_with_options(
        BorrowedRecord::new(event.record.key(), Some(&new_record.as_bytes())),
        WriteOptions::to_topic("unprocessed"),
    )?;
    Err("Invalid JSON in content field".into())
}
