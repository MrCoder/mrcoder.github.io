# Mermaid Gotchas

LLMs generally know Mermaid syntax well. This file covers only common mistakes and non-obvious behavior.

## Special Characters in Labels

Node IDs can't have spaces. Labels with special chars need quotes:

```mermaid
flowchart TD
    A["Label with: colons"]
    B["Label with (parens)"]
```

## Forgetting `end`

Every `alt`, `opt`, `loop`, `par`, `critical`, and `subgraph` block needs `end`:

```mermaid
sequenceDiagram
    alt Success
        A->>B: OK
    else Failure
        A->>C: Error
    end
```

## ER Diagram Cardinality

The symbols are non-obvious and easy to mix up:

```
||--||   Exactly one to exactly one
||--o{   One to zero-or-more
||--|{   One to one-or-more
}o--o{   Zero-or-more to zero-or-more
```

The `o` means "zero" (optional), `|` means "one" (required), `{` means "many".

## Class Diagram Generic Types

Use `~T~` not `<T>` — angle brackets break the parser:

```mermaid
classDiagram
    class List~T~ {
        +add(T item)
        +get(int index) T
    }
```

## Flowchart Direction

`TD` and `TB` are identical (top-down). `LR` works better for sequential/timeline flows.

## Sequence Diagram Activation Shorthand

`+` and `-` on arrows control activation bars:

```mermaid
sequenceDiagram
    A->>+B: Request (activates B)
    B-->>-A: Response (deactivates B)
```

## Theme Configuration

Goes BEFORE the diagram type declaration:

```mermaid
%%{init: {'theme':'forest'}}%%
flowchart TD
    A --> B
```
