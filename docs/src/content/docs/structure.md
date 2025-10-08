---
title: Bindings Structure
prev: false
next: false
head:
  - tag: style
    content: |
      .code-comparison {
        display: grid;
        grid-template-columns: 1fr 1fr;
        grid-template-rows: auto 0.5rem auto;
        column-gap: 1rem;
      }

      .code-comparison > div {
        margin: 0;
        overflow-x: hidden;
      }

      .code-comparison .title-left {
        grid-column: 1;
        grid-row: 1;
        font-weight: bold;
      }

      .code-comparison .title-right {
        grid-column: 2;
        grid-row: 1;
        font-weight: bold;
      }

      .code-comparison .code-left {
        grid-column: 1;
        grid-row: 3;
      }

      .code-comparison .code-right {
        grid-column: 2;
        grid-row: 3;
      }

      @media (max-width: 768px) {
        .code-comparison {
          grid-template-columns: 1fr;
          grid-template-rows: auto auto 0.5rem auto auto;
        }

        .code-comparison .title-left {
          grid-column: 1;
          grid-row: 1;
        }

        .code-comparison .title-right {
          grid-column: 1;
          grid-row: 4;
        }

        .code-comparison .code-left {
          grid-column: 1;
          grid-row: 2;
        }

        .code-comparison .code-right {
          grid-column: 1;
          grid-row: 5;
        }
      }
---

The tool writes the files in the specified output directory with the following structure. Assuming you have a `<service-name>.did` file, `<service-name>` is used for the generated files.

> Note: The generated code imports elements from `@icp-sdk/core`. You must install it, see [js.icp.build/core](https://js.icp.build/core).

## `<service-name>.ts`

This file contains the TypeScript wrapper for the Candid JS bindings generated in [`declarations/<service-name>.did.js`](#declarationsservice-namedidjs). It offers a more idiomatic and type-safe TypeScript interface over the Candid JS bindings.

Set the [`output.actor.disabled`](./core/api/type-aliases/GenerateOutputOptions.md#disabled) option to `true` to skip generating this file.

The generated file exposes:

- The [types](#types), that is the TypeScript representation of the Candid types.
- The [`<service-name>Interface` type](#service-nameinterface-type), that is the TypeScript interface for the service.
- The [`<service-name>` class](#service-name-class), that is the TypeScript class for the service.
- The [`createActor` function](#createactor-function), that creates a new instance of the actor.

### Types

This section contains the TypeScript representation of the Candid types. It contains all the types that are defined as Candid types in the `.did` file. To make the generated types more idiomatic, some types are transformed into more TypeScript-friendly types.

#### Options

Candid [options](https://github.com/dfinity/candid/blob/master/spec/Candid.md#options) are represented as a union of the inner option type and `null`:

<div class="code-comparison">

<div class="title-left">Candid</div>

<div class="code-left">

```
type MyType = opt text;
```

</div>

<div class="title-right">TypeScript</div>

<div class="code-right">

```typescript
type MyType = string | null;
```

</div>

</div>

#### Nested/Recursive Options

Nested or recursive options are represented as union of the `Some` and `None` types, where `Some` and `None` are defined as:

```typescript
interface Some<T> {
  __kind__: "Some";
  value: T;
}
interface None {
  __kind__: "None";
}
```

##### Nested

<div class="code-comparison">

<div class="title-left">Candid</div>

<div class="code-left">

```
type MyType = opt opt text;
type MyType2 = opt opt opt text;
```

</div>

<div class="title-right">TypeScript</div>

<div class="code-right">

```typescript
type MyType = Some<string | null> | None;
type MyType2 = Some<Some<string | null> | None> | None;
```

</div>

</div>

##### Recursive

<div class="code-comparison">

<div class="title-left">Candid</div>

<div class="code-left">

```
type A = B;
type B = opt A;
```

</div>

<div class="title-right">TypeScript</div>

<div class="code-right">

```typescript
type A = B;
type B = Some<A> | None
```

</div>

</div>

#### Record Fields with Options

Record fields that have an option type are optional fields in the TypeScript type:

<div class="code-comparison">

<div class="title-left">Candid</div>

<div class="code-left">

```
type MyType = record { 
  my_field : opt text;
}
```

</div>

<div class="title-right">TypeScript</div>

<div class="code-right">

```typescript
type MyType = {
  my_field?: string;
};
```

</div>

</div>

#### Variants

Candid [variants](https://github.com/dfinity/candid/blob/master/spec/Candid.md#variants) without type parameters are represented as TypeScript enums:

<div class="code-comparison">

<div class="title-left">Candid</div>

<div class="code-left">

```
type MyType = variant { 
  A; 
  B; 
}
```

</div>

<div class="title-right">TypeScript</div>

<div class="code-right">

```typescript
enum MyType {
  A,
  B,
}
```

</div>

</div>

#### Variants with Types

Variants that contain types in their fields are represented as TypeScript unions:

<div class="code-comparison">

<div class="title-left">Candid</div>

<div class="code-left">

```
type MyType = variant { 
  A : text; 
  B;
  C : record {
    my_field : text;
  };
}
```

</div>

<div class="title-right">TypeScript</div>

<div class="code-right">

```typescript
type MyType =
  | { __kind__: "A"; A: string }
  | { __kind__: "B"; B: null }
  | { __kind__: "C"; C: { my_field: string } };
```

</div>

</div>

### `<service-name>Interface` type

This type is the TypeScript interface for the service. It contains all the methods that are defined in the [Candid service](https://github.com/dfinity/candid/blob/master/spec/Candid.md#services) in the `.did` file.

For example, a Candid service will be represented as:

<div class="code-comparison">

<div class="title-left">Candid</div>

<div class="code-left">

```txt title="hello_world.did"
service : () -> {
  greet : (name : text) -> text;
};
```

</div>

<div class="title-right">TypeScript</div>

<div class="code-right">

```typescript title="hello_world.ts"
interface helloWorldInterface = {
  greet: (name: string) => Promise<string>;
};
```

</div>

</div>

### `<service-name>` class

This class implements the [`<service-name>Interface` type](#service-nameinterface-type). It can be instantiated with the [`createActor` function](#createactor-function).

For example, a Candid service will be represented as:

<div class="code-comparison">

<div class="title-left">Candid</div>

<div class="code-left">

```txt title="hello_world.did"
service : () -> {
  greet : (name : text) -> text;
};
```

</div>

<div class="title-right">TypeScript</div>

<div class="code-right">

```typescript title="hello_world.ts"
class HelloWorld implements helloWorldInterface {
  constructor(
    private actor: ActorSubclass<_SERVICE>,
  ) {}
  async greet(arg0: string): Promise<string> {
    const result = await this.actor.greet(arg0);
    return result;
  }
}
```

</div>

</div>

Where the `_SERVICE` type is imported from the [`declarations/<service-name>.did.d.ts`](#declarationsservice-namediddts) file and the [`ActorSubclass`](https://js.icp.build/core/latest/libs/agent/api/type-aliases/actorsubclass/) type is imported from the [`@icp-sdk/core/agent`](https://js.icp.build/core/latest/libs/agent/) module.

### `createActor` function

Creates an instance of the [`<service-name>` class](#service-name-class).

Here's an example of how to use the generated client:

```typescript
import { createActor } from "./bindings/hello_world";

const actor = createActor("your-canister-id");
const greeting = await actor.greet("World");
```

The signature of the `createActor` function is:

```typescript
interface CreateActorOptions {
    agent?: Agent;
    agentOptions?: HttpAgentOptions;
    actorOptions?: ActorConfig;
}

function createActor(canisterId: string, options: CreateActorOptions = {}): <service-name>Interface;
```

If both the `agent` and `agentOptions` are provided, the `agentOptions` will be ignored and the `agent` will be used. Otherwise, a new [`HttpAgent`](https://js.icp.build/core/latest/libs/agent/api/classes/httpagent/) will be created using the `agentOptions` if provided.

If provided, the `actorOptions` will be passed to the [`Actor.createActor`](https://js.icp.build/core/latest/libs/agent/api/classes/actor/#createactor) function. Otherwise, the default options will be used.

## `declarations/`

This folder contains the actual Candid JS bindings. It generates the same bindings that the [`dfx generate`](https://internetcomputer.org/docs/building-apps/developer-tools/dfx/dfx-generate) command was generating.

### `declarations/<service-name>.did.d.ts`

This file is used in TypeScript projects to type the Candid JS bindings generated in [`declarations/<service-name>.did.js`](#declarationsservice-namedidjs).

### `declarations/<service-name>.did.js`

This file contains the actual Candid JS bindings, that allow encoding and decoding JS objects to and from Candid.

## Optional files

### `<service-name>.d.ts`

This file contains the same TypeScript types as [`<service-name>.ts`](#service-namets). It is typically used to add to LLMs' contexts' to give knowledge about what types are available in the service. Set the [`output.actor.interfaceFile`](./core/api/type-aliases/GenerateOutputOptions.md#interfaceFile) option to `true` to generate this file.
