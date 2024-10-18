# RedpandaSovereignStructure

Reliably turning unstructured data into schema-conformant structed data with contextual categorization using Redpanda Connect and Redpanda Data Transforms.

Submission to https://redpanda-hackathon.devpost.com/

## Bottom line up front

Sovereign Structure reliably turns unstructured data into JSON-schema conformant structured data using LLMs that run directly in Redpanda, giving you the choice of model, and never requiring your data to leave your infrastructure.

## How it works

[Short video walkthrough](https://www.youtube.com/watch?v=y1A9489Bz9c)

1. Ingest unstructured data into the `input` topic
2. Use the `format` transform to wrap that data with a retry counter (`attempts`), sending to the `unprocessed` topic
3. Perform inference on records from the `unprocessed` topic, outputting to the `unverified` topic (with some light bloblang manipulation to recover the retry counter record structure)
4. `validation` transform reads from the `unprocessed` topic and will either:
   1. Write the record to the `structured` topic if the JSON is valid and conforms to the schema
   2. Rewrite the record to the `unprocessed` topic and increments the attempt counter if less than the threshold
   3. Write the record to the `unprocessable` topic if the retry threshold is met.

Diagram:

![image (4)](/assets/image%20(4).png)

In the provided example, we use LLMs to structure incoming emails into a [JSON schema](./schemas/email_schema.json), as well as categorizing them for our inbox.

Example input:
```
from: hackathonsubmitter@danthegoodman.com
to: hackathonsubmissions@redpanda.com
subject: i haz submission
body: isn't it great?!!
```

Example resulting record:

```
{
  "attempt": 2,
  "content": "from: hackathonsubmitter@danthegoodman.com\\nto: hackathonsubmissions@redpanda.com\\nsubject: i haz submission\\nbody: isn't it great?!!",
  "output": {
    "body": "isn't it great?!!",
    "category": "Primary",
    "from_addr": "hackathonsubmitter@danthegoodman.com",
    "from_name": "hackathonsubmitter",
    "subject": "i haz submission"
  }
}
```

This example is a bit more structured because of the performance of smaller models, which we'll dive into in the [Selecting an LLM](#selecting-an-llm) section.

## Running it

Check the `running` directory. In there you will find numbered scripts that you can execute in order:

```
zsh running/0-setup.sh
```

You'll then want to run:
```
zsh running/1-consume.sh
```
in a terminal to consume final output.

In a third terminal you can run:
```
zsh running/2-write.sh
```
to write records to the input topic.

Note that the first time you produce a record it will have to download the llama model.

You can also write your own records via stding with:

```
zsh running/3-write-stdin.sh
```

**This can take some time.** Use `docker compose logs -f redpanda-connect` to see what it's up to!

The terminal running the `1-consume.sh` script will spit out records that are formatted as JSON, and fulfill the example task. You can also use the redpanda console at `localhost:8080` and inspect records in the various topics. There are some example emails in the `records` directory you can try sending.

_Note that it can also take some time to see the models run through the whole pipeline, depending on model size and hardware._

## Motivation

OpenAI structured outputs are super useful, unlocking many novel use cases for LLMs, yet we seldom have the luxuries of managed models with open source models that we can run locally.

Sovereign Structure give us back the structured outputs feature of OpenAI models while also giving us all the advantages that [Redpanda Sovereign AI promises](https://ai.redpanda.com/).

While a system that can directly follow [the design shared by OpenAI](https://openai.com/index/introducing-structured-outputs-in-the-api/#:~:text=achieve%20100%25%20reliability.-,Constrained%20decoding,-Our%20approach%20is) might reduce the complexity of the pipeline and result in fewer errors landing in the `unrpocessable` topic, there are some advantages to this system.

First, the following quote from OpenAI gives pause:

> However, once the model has already sampled `{“val`, then `{` is no longer a valid token

Well... `{` is still valid. It can be part of the string :)

While this may be a poorly contrived example, it may not be, so we don’t fully know the limitations of their JSON output (e.g. do they support `null` as the output, or a top-level array?)

One might think that the provided examples are quite contrived, and that this is a cheap clone of the [existing structured outputs demo](https://www.redpanda.com/blog/ai-connectors-gpu-runtime-support) (which is where the demo task comes from). However, if you talk to enterprise customers, you've probably already guessed where I'm about to take this, any why this solution is so valuable. Sovereign structure enables enterprises to:

1. Reduce costs for both egress and expensive managed inference
2. Keep their data in their environment. No more shipping sensitive data to OpenAI (which is often a non-starter for enterprise customers)
3. Choice of model:
    1. Balancing of throughput, accuracy, and resource consumption
    2. Ability to use differnet models that may perform better for certain tasks
    3. Use of proprietary models fine-tuned for their workloads

A solution that achieves all of these with such simplicity does not exist in the industry right now, and Redpanda Connect + Data Transforms enables this. This level of flexbility is critical to support enterprise AI use cases.

## Limitations

This system is not perfect, there are a few unoptimal solutions that have to be performed:

1. There are a few places where build-time variables would have to be injected, because they are not something that can be resolved (conveniently at least) at runtime. For example the schema registry IDs in the data transforms (there may be a way to resolve these then cache them with the schema registry sdk).
2. There is no (convenient) way to dynamically pull schema registry entries into the connect LLM prompt at the moment, so we hard code it in
3. Smaller LLMs are not smart enough to be given the JSON schema directly, so the prompt included an example final JSON output with comments about what is optional.

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

Without a GPU it can take a few seconds (after the model is initially loaded) depending on record size and throughput. Thankfully we can parallelize operations with more cores, and we can introduce GPU machines to wildly speed up inference time.

The novel retry framework shows it's immense value in practice:

```
{
    "attempts": 1,
    "content": "from: hackathonsubmitter@danthegoodman.com\\nto: hackathonsubmissions@redpanda.com\\nsubject: i haz submission\\nbody: isn't it great?!!",
    "output": {
        "body": "isn't it great?!!",
        "category": "Social",
        "from_addr": "hackathonsubmitter@danthegoodman.com",
        "from_name": null,
        "subject": "i haz submission"
    }
}
```

Notice the `"attempts": 1`?

As you can see from this example, the _first_ record produced into the pipeline actually had to retry before it produced valid JSON that conformed to the JSON schema. Without the retry framework, total failures would be common with these small LLMs.

The original record looked like:

```
{
    "attempts": 0,
    "content": "from: hackathonsubmitter@danthegoodman.com\\nto: hackathonsubmissions@redpanda.com\\nsubject: i haz submission\\nbody: isn't it great?!!",
    "output": {
        "body": "isn't it great?!!",
        "from_addr": "hackathonsubmitter@danthegoodman.com",
        "subject": "i haz submission"
    }
}
```

This is mmissing the required `category` field. Thankfully the schema validation detected this, and sent it back through again to retry.

Ensuring that records conforms to an expected JSON schema is _critical_ for production workloads.

While we exchange accuracy for speed and memory consumption by using small LLMs, we compensate with the retry framework that negates the downsides at a cost generally lower than using larger models that higher zero-shot accuracy.

**Update**: Using the same Typescript-JSON hybrid declaration format has proven WILDLY more accurate, especially with smaller models (thanks BAML founders).

Those promps should be in the format (see full example in [`structured.yml`](./connect/structured.yml)):

```
Your task is to {{ task }}, and output it as JSON

Extract this information:
--
${!this.content}
--

Answer in JSON using the following schema:
{
  subject: string or null,
  from_name: string or null,
  from_addr: string,
  body: string,
  category: "Primary" or "Social" or "Promotions" or "Updates" or "Forums" or "Support"
}

For the category, use the following guide to help your decision

Primary: Emails from people you know and messages that don’t appear in other tabs.
Social: Messages from social networks and media-sharing sites.
...

If a field from the schema is missing from the email, omit the JSON property rather than putting something blank in or hallucinating.
```

### Selecting an LLM

I've left `llama3.2:3b` as the initial model since with semi-structured inputs (like [`example_email.txt`](./example_email.txt)), it can _sometimes_ provide conformant output, and was good enough for development. However to use truly unstructured outputs (blobs of text), you need at least a 20x larger model (70b and up).

Unfortunately, smaller models are quite bad at JSON output, as well as generally understanding unstructured to structured conversions. Larger models quickly become more accurate and consistent with their outputs. As you can see in the example above, smaller models will hallucinate fields rather than omitting them.

For consistent production-level performance, `llama3.1:70b` or larger is required, running on GPU instances. However this is hardly an inconvenience for enterprise users in comparison to shipping data to OpenAI. While maxed-out macbook pros can run this, it INSTANTLY cooks the laptop, so I would not suggest testing that outside a GPU server.

## Gotchas and other notes

You will need to adjust the schema registry ID for `record_attempted` in the `format` and `validation` rust transforms if you're not using this vanilla demo environment. This is possible to customize with build flags, but that adds an unncessary amount of complexity for a demo like this.

The connect pipeline specifies the `json` output format. This works fine, but `text` is also supported, as the transform will cast JSON string to JSON if it is given a string directly.

## Future work

- Exploring more performant WASM allocators for rust transforms
- The memory allocated to the WASM transforms has been bumped to 10MB because somehow something was very memory hungry?
- Framework for evaluating best model for a given task
- Improve error handling and reliability: Right now it doesn't handle things like blank records very well (trasnform will just crash from invalid JSON)
- Improve the prompt, it's hard to iterate with small models on a laptop. For a production use case, using a large GPU machine with larger models and a wide variety of inputs to test should be performed to determine the proper model.
- Integrating the BAML parser. They've done a wonderful job in parsing partially broken outputs, which would be a more efficient mechanism than just straight retrying. The core is written in Rust, so assuming no dependency issues, it should compile to WASM.

Redpanda really is wildly easier to use than similar solutions in both the streaming and inference space. This, combined with great support in the community Slack, allowed me to rapidly iterate on this project.
