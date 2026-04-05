# LLM Parameters

## Overview

LLM parameters control how the AI model generates responses. The SDK provides two parameter sets:

- **Prompt LLM params** -- control the main conversation model
- **Post-prompt LLM params** -- control the summary/analysis model after the call

## Setting Parameters

```rust
// Main conversation
agent.set_prompt_llm_params(json!({
    "temperature": 0.3,
    "top_p": 0.9,
    "barge_confidence": 0.6,
    "presence_penalty": 0.0,
    "frequency_penalty": 0.1,
}));

// Post-call summary
agent.set_post_prompt_llm_params(json!({
    "temperature": 0.1,
}));
```

## Parameter Reference

### temperature

Controls randomness in responses. Lower values produce more deterministic output.

| Value | Behaviour |
|-------|-----------|
| 0.0 - 0.3 | Precise, consistent, factual |
| 0.4 - 0.7 | Balanced (default range) |
| 0.8 - 1.0 | Creative, varied, exploratory |

### top_p

Nucleus sampling threshold. Controls the cumulative probability of token selection.

| Value | Behaviour |
|-------|-----------|
| 0.8 - 0.9 | Focused token selection |
| 0.95 - 1.0 | Broader selection (default) |

### barge_confidence

Confidence threshold for barge-in detection (caller interrupting the AI).

| Value | Behaviour |
|-------|-----------|
| 0.1 - 0.3 | Easy to interrupt (casual conversations) |
| 0.5 | Default |
| 0.7 - 0.9 | Hard to interrupt (let the AI finish important info) |

### presence_penalty

Penalises tokens that have appeared in the conversation. Reduces repetition of topics.

| Value | Behaviour |
|-------|-----------|
| 0.0 | No penalty (default for technical agents) |
| 0.3 - 0.6 | Moderate variety |
| 1.0+ | Strong topic shifting |

### frequency_penalty

Penalises tokens proportional to how often they appear. Reduces word repetition.

| Value | Behaviour |
|-------|-----------|
| 0.0 | No penalty |
| 0.1 - 0.3 | Slight word variety |
| 0.5+ | Strong vocabulary diversification |

## Personality Presets

### Precise Technical Agent

```rust
agent.set_prompt_llm_params(json!({
    "temperature": 0.2,
    "top_p": 0.85,
    "barge_confidence": 0.8,
    "presence_penalty": 0.0,
    "frequency_penalty": 0.1,
}));
```

### Creative Conversational Agent

```rust
agent.set_prompt_llm_params(json!({
    "temperature": 0.8,
    "top_p": 0.95,
    "barge_confidence": 0.3,
    "presence_penalty": 0.4,
    "frequency_penalty": 0.3,
}));
```

### Balanced Customer Service

```rust
agent.set_prompt_llm_params(json!({
    "temperature": 0.5,
    "top_p": 0.9,
    "barge_confidence": 0.5,
    "presence_penalty": 0.1,
    "frequency_penalty": 0.1,
}));
```

## AI Model Selection

```rust
agent.set_params(json!({
    "ai_model": "gpt-4.1-nano",  // default
}));
```

The `ai_model` parameter selects the underlying model. This is set via `set_params()`, not `set_prompt_llm_params()`.

## End-of-Speech Timeout

Controls how long the platform waits after the caller stops speaking before treating it as end of input:

```rust
agent.set_params(json!({
    "end_of_speech_timeout": 500,  // 500ms
}));
```

Lower values make the agent respond faster. Higher values accommodate pauses in speech.

## Attention Timeout

How long the platform waits for the caller to speak before timing out:

```rust
agent.set_params(json!({
    "attention_timeout": 15000,  // 15 seconds
}));
```
