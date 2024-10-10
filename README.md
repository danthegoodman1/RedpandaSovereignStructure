# RedpandaSovereignStructure

Turning unstructured data into structed data using Redpanda Connect and Redpanda Data Transforms.

Submission to https://redpanda-hackathon.devpost.com/

## Bottom line up front

Turns unstructured data into structured data using local models that run directly in Redpanda, so your data never has to leave.

## Motivation

OpenAI structured outputs are super useful, unlocking many novel use cases for LLMs, yet we seldom have the luxuries of managed models with open source models that we can run locally.

This achieves that.

It gives us all the advantages that [Redpanda Sovereign AI promises](https://ai.redpanda.com/), while also providing the benefits of structured outputs.

While a system that can directly follow [the design shared by OpenAI](https://openai.com/index/introducing-structured-outputs-in-the-api/#:~:text=achieve%20100%25%20reliability.-,Constrained%20decoding,-Our%20approach%20is) might reduce the complexity of the pipeline and result in fewer errors landing in the DLQ, there are some advantages.

First, the following quote from OpenAI gives pause:

> However, once the model has already sampled `{“val`, then `{` is no longer a valid token

Well... `{` is still valid. It can be part of the string :)

While this may be a poorly contrived example, it may not be, so we don’t fully know the limitations of their JSON output (e.g. do they support `null` as the output, or a top-level array?)

What we trade in some runtime complexity to coordinate the AI and transform stages is orders of magnitude less complex and costly than what OpenAI has done, and now we can colocate the model with our data!

No more shipping our sensitive data out of our network (expensive egress) and to OpenAI who does who knows with it!

## Limitations

This system is not perfect, there are a few unoptimal solutions that have to be performed:

1. Because the LLM cannot guarantee JSON output, we must send it to the subsequent transform as a string for validation.
2. Lack of dynamic injection of schema variable into data transforms (at least that I could find). You will need to adjust the schema registry ID for `record_attempted` in the `format` and `validation` rust transforms.

## Code structure

Expected dependencies to be installed are:
- Docker
- Rust toolchain (rustup, cargo, etc.)
- A unix-based shell env (developed with zsh on arm macOS)

Everything should work out of the box. If something breaks first run, please raise an issue!

- [`running`](./running/), you will find the various scripts needed to execute the code.
- [`transforms`](./transforms/) you will find the various Rust data transforms that are used
- [`helpers`](./helpers) are just various helper scripts I used to develop, tune, and eval the project, and are not required for execution or evaluation

## Running it

Check the `running` directory. In there you will find numbered scripts that you can execute in order:

```
bash running/0-setup.sh
```

## How it works

TODO: add diagram

## Gotchas and other notes

you will need to adjust the schema registry ID for `record_attempted` in the `format` and `validation` rust transforms
