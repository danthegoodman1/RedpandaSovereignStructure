# RedpandaSovereignStructure

Turning unstructured data into structed data using Redpanda Connect and Redpanda Data Transforms.

Submission to https://redpanda-hackathon.devpost.com/

## Bottom line up front

Turns unstructured data into structured data using local models that run directly in Redpanda, so your data never has to leave your infrastructure. Combined with the retry mechanism, unstructured data to structured outputs becomes reasonably reliable for production workloads.

## How it works

1. Ingest unstructured data into the `input` topic
2. Use the `format` transform to wrap that data with a retry counter, sending to the `unprocessed` topic
3. Perform inference on records from the `unprocessed` topic, outputting to the `unverified` topic (with some light bloblang manipulation to recover the retry counter record structure)
4. `validation` transform reads from the `unprocessed` topic and will either:
   1. Write the record to the `structured` topic if the JSON is valid and conforms to the schema
   2. Rewrite the record to the `unprocessed` topic and increments the attempt counter if less than the threshold
   3. Write the record to the `unprocessable` topic if the retry threshold is met.

Diagram:

![image (4)](/assets/image%20(4).png)

## Running it

Check the `running` directory. In there you will find numbered scripts that you can execute in order:

```
zsh running/0-setup.sh
```

You'll then want to run:
```
running/1-consume.sh
```
in a terminal to consume final output.

In a third terminal you can run:
```
running/2-write.sh
```
to write records to the input topic.

Note that the first time you produce a record it will have to download the llama model.

**This can take some time.** Use `docker compose logs -f redpanda-connect` to see what it's up to!

The terminal running the `1-consume.sh` script will spit out records that are formatted as JSON, and fulfill the example task. You can also use the redpanda console at `localhost:8080` and inspect records in the various topics. There are some example emails in the `records` directory you can try sending.

_Note that it can also take some time to see the models run through the whole pipeline, depending on model size and hardware._

## Motivation

OpenAI structured outputs are super useful, unlocking many novel use cases for LLMs, yet we seldom have the luxuries of managed models with open source models that we can run locally.

This achieves that.

It gives us all the advantages that [Redpanda Sovereign AI promises](https://ai.redpanda.com/), while also providing the benefits of structured outputs.

While a system that can directly follow [the design shared by OpenAI](https://openai.com/index/introducing-structured-outputs-in-the-api/#:~:text=achieve%20100%25%20reliability.-,Constrained%20decoding,-Our%20approach%20is) might reduce the complexity of the pipeline and result in fewer errors landing in the DLQ, there are some advantages to this system.

First, the following quote from OpenAI gives pause:

> However, once the model has already sampled `{“val`, then `{` is no longer a valid token

Well... `{` is still valid. It can be part of the string :)

While this may be a poorly contrived example, it may not be, so we don’t fully know the limitations of their JSON output (e.g. do they support `null` as the output, or a top-level array?)

One might think that the provided examples are quite contrived, and that this is a cheap clone of the [existing structured outputs demo](https://www.redpanda.com/blog/ai-connectors-gpu-runtime-support) (which is where the demo task comes from). However if you talk to enterprise customers, you'll understand you've probably already guessed where I'm about to take this, any why this solution is so valuable. This enables enterprises to:

1. Reduce costs (both egress and expensive managed inference)
2. Keep our data in our environment (no more shipping sensitive data to OpenAI, which is often a non-starter for enterprise customers)
3. Choice of model:
    1. Balancing of throughput, accuracy, and resource consumption
    2. Ability to use differnet models that may perform better for certain tasks
    3. Use of proprietary models fine-tuned for their workloads

A solution that achieves all of these with such simplicity does not exist in the industry right now, and Redpanda Connect + Data Transforms enables this.

## Limitations

This system is not perfect, there are a few unoptimal solutions that have to be performed:

1. Because the LLM cannot guarantee JSON output, we must send it to the subsequent transform for JSON and schema validation.
2. There are a few places where build-time variables would have to be injected, because they are not something that can be resolved (conveniently at least) at runtime. For example the schema registry IDs in the data transforms (there may be a way to resolve these then cache them with the schema registry sdk).
3. There is no (convenient) way to dynamically pull schema registry entries into the connect LLM prompt at the moment, so we hard code it in

## Code structure

Expected dependencies to be installed are:
- Docker
- Rust toolchain (rustup, cargo, etc.)
- A unix-based shell env (developed with zsh on arm macOS, but other shells should work fine)

Everything should work out of the box. If something breaks first run, please raise an issue!

- [`running`](./running), you will find the various scripts needed to execute the code.
- [`transforms`](./transforms) you will find the various Rust data transforms that are used.
- [`helpers`](./helpers) are just various helper scripts I used to develop, tune, and eval the project, and are not required for execution or evaluation.
- [`records`](./records) Some example records for testing.

## Real world performance

Without a GPU it can be pretty slow.

However, the novel retry framework shows it's immense value in practice:

```
dangoodman: ~/code/RedpandaSovereignStructure git:(main) zsh running/1-consume.sh
Consuming from 'structured' topic...
{
  "topic": "structured",
  "value": "{\"attempts\":2,\"content\":\"from: hackathonsubmitter@danthegoodman.com\\\\\\\\nto: hackathonsubmissions@redpanda.com\\\\\\\\nsubject: i haz submission\\\\\\\\nbody: isn't it great?!!\",\"output\":{\"body\":\"isn't it great?!!\",\"category\":\"Primary\",\"from_addr\":\"hackathonsubmitter@danthegoodman.com\",\"from_name\":\"hackathonsubmitter\",\"subject\":\"i haz submission\"}}",
  ...
}
```

Notice the `\"attempts\":2`?

As you can see from this example, the first record produced into the pipeline actually had to retry _twice_ before it produced valid JSON that conformed to the JSON schema. Without the retry framework, total failures would be common with these small LLMs.

Additionally, ensuring that it conforms to an expected JSON schema is critical for production workloads.

While we exchange accuracy for speed and memory consumption by using small LLMs, we compensate with the retry framework that negates the downsides at a cost generally lower than using larger models that higher zero-shot accuracy.

## Gotchas and other notes

You will need to adjust the schema registry ID for `record_attempted` in the `format` and `validation` rust transforms if you're not using this vanilla demo environment. This is possible to customize with build flags, but that adds an unncessary amount of complexity for a demo like this.

The connect pipeline specifies the `json` output format. This works fine, but `text` is also supported, as the transform will cast JSON string to JSON if it is given a string directly.

The llama3.2 3b model is pretty _meh_. 3B params plus quantization really doesn't do many favors, but it can work most of the time. If you have the memory (and the GPUs), `model: llama3.1:8b` is a good next option to try. Microsoft `phi3.5` and `phi3:14b` models are also really impressive for their size! llama 3.2 3b and phi3.5 trade punches in benchmarks, so test what works best for you:

![461157789_931406385491961_1692349435372036848_n](/assets/461157789_931406385491961_1692349435372036848_n.png)

## Future work

- Exploring more performant WASM allocators for rust transforms
- The memory allocated to the WASM transforms has been bumped to 10MB because somehow something was very memory hungry?
- Framework for evaluating best model for a given task
