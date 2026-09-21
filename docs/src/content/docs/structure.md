---
title: Bindings Structure
prev: false
next: false
tableOfContents:
  maxHeadingLevel: 4
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

`<service-name>` is the `.did` file name. Characters that are not legal in an identifier are
replaced with `_` where the name is used as one, so `my-backend.did` and `my.backend.did` both
yield the same identifiers as `my_backend.did` would. A `#`, `?`, `%` or `\` in the file name is
refused instead: the wrapper imports the declarations by that name, a module specifier is
resolved as a URL, and no spelling of it resolves the file in both Node and a bundler. Only
the actor files carry such an import, so the name is accepted whenever they are not produced:
with `output.actor.disabled`, or for a `.did` without a `service`.

Set the [`output.actor.disabled`](./core/api/type-aliases/GenerateOutputOptions.md#disabled) option to `true` to skip generating this file. A `.did` that declares types but no `service` has nothing to wrap, so it produces the declarations only, as if that option were set.

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

##### Through a name

A name standing for the inner option nests just as a spelled-out `opt opt` does — the two are
one Candid type and carry the same three states:

<div class="code-comparison">

<div class="title-left">Candid</div>

<div class="code-left">

```
type Inner = opt text;
type MyType = opt Inner;
```

</div>

<div class="title-right">TypeScript</div>

<div class="code-right">

```typescript
type Inner = string | null;
type MyType = Some<Inner> | None;
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

A field of `opt opt T` is `my_field?: T | null`: omitting it (or `undefined`) is the outer
option absent, `null` is present with an absent inner value, and a value is present. Deeper
nesting keeps the outer level as `?` and represents the rest as `Some` / `None`, as a
standalone value of that type would be. A field whose inner option is reached through a name
is the same Candid type as the spelled-out one, and is represented the same way.

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

A payload of `opt T` is `T | null`, `null` being the absent payload. A payload of `opt opt T`
has no slot of its own for the outer level, so it is typed as a standalone nested option,
`Some<T | null> | None`, exactly as an `opt opt T` argument or result would be. A payload
whose inner option is reached through a name is the same Candid type as the spelled-out one,
and is typed the same way.

### `<service-name>Interface` type

This type is the TypeScript interface for the service. It contains all the methods that are defined in the [Candid service](https://github.com/dfinity/candid/blob/master/spec/Candid.md#services) in the `.did` file.

`<service-name>` is the basename of the `.did` file. Characters that cannot appear in a generated identifier are replaced with a single `_` per run, so `my-backend.did` produces `my_backendInterface` and the `My_backend` class — the same names as if the file had been called `my_backend.did`. The generated declarations are still imported under the original filename.

Accepted characters are ASCII letters, digits, `_` and `$`, the zero-width non-joiner and joiner, plus Unicode characters in `XID_Start`/`XID_Continue`. That is marginally stricter than TypeScript's own identifier grammar, so a small number of uncommon Unicode characters are replaced even though TypeScript would have accepted them.

The class name is additionally suffixed with `_` when capitalizing it would shadow a JavaScript built-in, so `map.did` produces the class `Map_` rather than `Map`.

For example, a Candid service will be represented as:

<div class="code-comparison">

<div class="title-left">Candid</div>

<div class="code-left">

```txt title="hello_world.did"
service : {
  greet : (name : text) -> (text);
};
```

</div>

<div class="title-right">TypeScript</div>

<div class="code-right">

```typescript title="hello_world.ts"
interface hello_worldInterface {
  greet(name: string): Promise<string>;
}
```

</div>

</div>

### `<service-name>` class

This class implements the [`<service-name>Interface` type](#service-nameinterface-type). It can be instantiated with the [`createActor` function](#createactor-function). It keeps the actor it wraps in a private `#actor` field, so it is erasable-syntax TypeScript — usable under `erasableSyntaxOnly`, Node's type stripping and Deno — and needs a compilation target of ES2015 or later.

For example, a Candid service will be represented as:

<div class="code-comparison">

<div class="title-left">Candid</div>

<div class="code-left">

```txt title="hello_world.did"
service : {
  greet : (name : text) -> (text);
};
```

</div>

<div class="title-right">TypeScript</div>

<div class="code-right">

```typescript title="hello_world.ts"
class Hello_world implements hello_worldInterface {
  readonly #actor: ActorSubclass<_SERVICE>;
  constructor(actor: ActorSubclass<_SERVICE>) {
    this.#actor = actor;
  }
  async greet(arg0: string): Promise<string> {
    const result = await this.#actor.greet(arg0);
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

function createActor(canisterId: string, options: CreateActorOptions = {}): Hello_world;
```

If both the `agent` and `agentOptions` are provided, the `agentOptions` will be ignored and the `agent` will be used. Otherwise, a new [`HttpAgent`](https://js.icp.build/core/latest/libs/agent/api/classes/httpagent/) will be created using the `agentOptions` if provided.

If provided, the `actorOptions` will be passed to the [`Actor.createActor`](https://js.icp.build/core/latest/libs/agent/api/classes/actor/#createactor) function. Otherwise, the default options will be used.

## Names the generator cannot carry

Most candid names pass through untouched. Where a name would collide with something the
generated files themselves use, the generator escapes it; where two candid names would end
up as one, generation fails with a diagnostic rather than emitting something TypeScript would
merge silently.

### Type names

A candid type is declared under its own name, escaped with a trailing `_` when that name is a
reserved word, a JavaScript global, a name TypeScript cannot use as a type (`never`, `string`,
`as`, `keyof`, …), or something the generated module already occupies — the types it imports
from `@icp-sdk/core` and the `Option` / `Some` / `None` / `CreateActorOptions` helpers it
declares. So `type Option` becomes `Option_` and `type Map` becomes `Map_`.

The declarations files apply the same rule to the names they occupy themselves: `IDL`,
`Principal`, `ActorMethod`, their own exports (`_SERVICE`, `idlFactory`, `init`, `idlService`,
`idlInitArgs`), and the built-in types they print for a `vec` (`Array`, `Uint8Array` and the
other typed arrays). A candid type of one of those names is exported as `IDL_` or
`Uint8Array_`, and the wrapper imports it under that name.

Two candid types that escape to one exported name — `IDL` and `IDL_` — fail generation.

An inline all-null variant is declared as an enum named after its tags, `Variant_x` for
`variant { x }`. A named candid type of that name with the same tags shares the enum; with
different tags, the inline variant is declared as `Variant_x_` instead.

Two candid names that end up as one generated name — two inline variants whose tags join to
the same `Variant_…`, a type named like the actor class, a type named like a conversion
function — fail generation, since TypeScript would merge the declarations into a type
claiming members the value does not have.

### Parameter names

Candid argument names become the parameter names of the interface's method signatures and of
named `func` types. A name that is not an identifier is sanitized (`my-arg` becomes
`my_arg`), a reserved word gains a trailing `_` (`new` becomes `new_`), and an argument that is
unnamed, or whose name another argument already took, is called `argN` after its position —
suffixed with `_` while that name is taken as well, so `(arg1 : nat, nat)` gives `arg1` and
`arg1_`. The wrapper class always names its parameters `argN`. Callers pass arguments positionally,
so these names are documentation only.

### Method names

Method names are property keys, not declarations, so they are never escaped: a method keeps
exactly the name the `.did` gives it, quoted where it has to be. `"my-method"` and `"new"` are
both fine — the latter is quoted because a bare `new(): T` would declare the interface
*constructible* rather than declaring a method called `new`.

Three groups are refused on the actor class:

| method name | why |
| --- | --- |
| `constructor` | bare or quoted, a class member of that name *is* the constructor; as a computed key it overwrites `prototype.constructor` |
| `toString`, `valueOf`, `hasOwnProperty`, and the rest of `Object.prototype` | overriding these changes behaviour JavaScript relies on: `String(actor)` would fire a canister call and then throw |
| `then`, `toJSON` | a class with a `then` method is a thenable, so `await actor` or returning it from an `async` function would call it; `JSON.stringify(actor)` calls `toJSON` |

Only the wrapper class is subject to these. A nested `type S = service { … }` is typed as a
`Principal` in the wrapper, so its interface never describes a runtime object and its methods
may carry any name — except `__proto__`, which is refused everywhere (see below).

The first group is unreachable whichever way it is emitted. The other two are a deliberate
restriction rather than an impossibility: such a method would work if called
directly, but it overrides a protocol, and a network request should not be a side effect of
logging, awaiting or serializing the actor. If you need one of these names, the `.did` has to
rename the method; please open an issue if that is a real constraint for your canister.

### Refused names

A candid name of `__proto__` — as a record field, a variant tag or a method — cannot be
carried by any generated file: JavaScript treats it as the object's prototype wherever it
appears, so the member would be missing at runtime. Generation fails whichever files are
wanted. A variant that carries payloads gains a `__kind__` discriminant, so a tag of that
name is refused as well.

Every other refusal on this page concerns the actor files only. Generating the declarations
alone (`output.actor.disabled`, or `--actor-disabled` on the CLI) still succeeds.

## `declarations/`

This folder contains the actual Candid JS bindings. It generates the same bindings that the [`dfx generate`](https://internetcomputer.org/docs/building-apps/developer-tools/dfx/dfx-generate) command was generating.

> **Tip**: Set the [`output.declarations.flat`](./core/api/type-aliases/GenerateOutputOptions.md#flat) option to `true` (or pass `--declarations-flat` via the CLI) to write these files directly into `outDir` instead of a `declarations/` subfolder. This is useful for projects that need full control over the output file layout without post-processing scripts to move or rename files.

See the [Migrating](./migrating) page for more information on how to migrate from `dfx generate`.

### `declarations/<service-name>.did.d.ts`

This file is used in TypeScript projects to type the Candid JS bindings generated in [`declarations/<service-name>.did.js`](#declarationsservice-namedidjs). The exported types are:

#### `_SERVICE` type

This type is the TypeScript interface for the service. It contains all the methods that are defined in the [Candid service](https://github.com/dfinity/candid/blob/master/spec/Candid.md#services) in the `.did` file.

For example:

<div class="code-comparison">

<div class="title-left">Candid</div>

<div class="code-left">

```txt title="hello_world.did"
service : {
  greet : (text) -> (text);
};
```

</div>

<div class="title-right">TypeScript</div>

<div class="code-right">

```typescript title="declarations/hello_world.did.d.ts"
import type { ActorMethod } from '@icp-sdk/core/agent';
import type { IDL } from '@icp-sdk/core/candid';
import type { Principal } from '@icp-sdk/core/principal';

export interface _SERVICE { 'greet' : ActorMethod<[string], string> }
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
```

</div>

</div>

You can use it to type the [`Actor`](https://js.icp.build/core/latest/libs/agent/api/classes/actor/) instance with the methods of the service declared in the `.did` file:

```typescript
import { type _SERVICE, idlFactory } from "./bindings/declarations/hello_world.did";

const actor = Actor.createActor<_SERVICE>( idlFactory, {
  canisterId: "your-canister-id",
});

const greeting = await actor.greet("World"); // greet method is now available on the actor instance and typed
console.log(greeting);
```

Additionally, all the types used in the service class are exported as well:

<div class="code-comparison">

<div class="title-left">Candid</div>

<div class="code-left">

```txt title="hello_world.did"
type GreetArgs = record {
  name : text;
};

service : {
  greet : (GreetArgs) -> (text);
};
```

</div>

<div class="title-right">TypeScript</div>

<div class="code-right">

```typescript title="declarations/hello_world.did.d.ts"
import type { ActorMethod } from '@icp-sdk/core/agent';
import type { IDL } from '@icp-sdk/core/candid';
import type { Principal } from '@icp-sdk/core/principal';

export interface GreetArgs { 'name' : string } // <- exported type
export interface _SERVICE { 'greet' : ActorMethod<[GreetArgs], string> }
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
```

</div>

</div>

#### `idlFactory` function

The function signature is:

```typescript
const idlFactory: IDL.InterfaceFactory;
```

See [`IDL.InterfaceFactory`](https://js.icp.build/core/latest/libs/candid/api/namespaces/idl/type-aliases/interfacefactory/).

#### `init` function

The function signature is:

```typescript
const init: (args: { IDL: typeof IDL }) => IDL.Type[];
```

See [`IDL.Type`](https://js.icp.build/core/latest/libs/candid/api/namespaces/idl/classes/type/).

#### `idlService` type

> Note: This type is only exported if the [`output.declarations.rootExports`](./core/api/type-aliases/GenerateOutputOptions.md#rootExports) option is set to `true`.

The type signature is:

```typescript
const idlService: IDL.ServiceClass;
```

See [`IDL.ServiceClass`](https://js.icp.build/core/latest/libs/candid/api/namespaces/idl/classes/serviceclass/).

#### `idlInitArgs` type

> Note: This type is only exported if the [`output.declarations.rootExports`](./core/api/type-aliases/GenerateOutputOptions.md#rootExports) option is set to `true`.

The type signature is:

```typescript
const idlInitArgs: IDL.Type[];
```

See [`IDL.Type`](https://js.icp.build/core/latest/libs/candid/api/namespaces/idl/classes/type/).

### `declarations/<service-name>.did.js`

This file contains the actual Candid JS bindings, that allow encoding and decoding JS objects to and from Candid. This file exports two functions:

#### `idlFactory` function

Typically passed to the [`Actor.createActor`](https://js.icp.build/core/latest/libs/agent/api/classes/actor/#createactor) function:

```typescript
import { idlFactory } from "./bindings/declarations/hello_world.did";

const actor = Actor.createActor(idlFactory, {
  canisterId: "your-canister-id",
});
```

#### `init` function

Used to type the initialization arguments of the service.

You can use the [`output.declarations.rootExports`](./core/api/type-aliases/GenerateOutputOptions.md#rootExports) option to control whether to export the root types in the declarations JS file.

#### `idlService` type

> Note: This type is only exported if the [`output.declarations.rootExports`](./core/api/type-aliases/GenerateOutputOptions.md#rootExports) option is set to `true`.

This type is the same service class that the [`idlFactory` function](#idlfactory-function-1) returns.

Additionally, if the [`output.declarations.rootExports`](./core/api/type-aliases/GenerateOutputOptions.md#rootExports) option is set to `true`, all the types used in the service class are exported as constants from the declarations JS file.

Example:

<div class="code-comparison">

<div class="title-left">Candid</div>

<div class="code-left">

```txt title="hello_world.did"
type GreetArgs = record {
  name : text;
};

service : {
  greet : (GreetArgs) -> (text);
};
```

</div>

<div class="title-right">TypeScript</div>

<div class="code-right">

```typescript title="declarations/hello_world.did.js"
import { IDL } from '@icp-sdk/core/candid';

export const GreetArgs = IDL.Record({ 'name' : IDL.Text });

export const idlService = IDL.Service({
  'greet' : IDL.Func([GreetArgs], [IDL.Text], []),
});

export const idlInitArgs = [];

export const idlFactory = ({ IDL }) => {
  const GreetArgs = IDL.Record({ 'name' : IDL.Text });
  
  return IDL.Service({ 'greet' : IDL.Func([GreetArgs], [IDL.Text], []) });
};

export const init = ({ IDL }) => { return []; };
```

</div>

</div>

#### `idlInitArgs` type

> Note: This type is only exported if the [`output.declarations.rootExports`](./core/api/type-aliases/GenerateOutputOptions.md#rootExports) option is set to `true`.

This type is the same types that the [`init` function](#init-function-1) returns.

## Optional files

### `<service-name>.d.ts`

This file contains the same TypeScript types as [`<service-name>.ts`](#service-namets). It is typically used to add to LLMs' contexts' to give knowledge about what types are available in the service. Set the [`output.actor.interfaceFile`](./core/api/type-aliases/GenerateOutputOptions.md#interfaceFile) option to `true` to generate this file.
