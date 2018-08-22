# Errors

## Error handling in Rust
A quick summary of the common pattern:

Every function in rust that returns `Result<(SOMETHING)>` will basically return either an `Ok(SOMETHING)` or an `Err(SOME_ERROR_TYPE)`. Typically, this is done invisibly by adding a questionmark `?` to the end of an operation that can fail.

This is used extensively, and errors tend to propagate all the way up to main that handles it and does the actual `process::exit(rc)`.

If you want to deal with a specific error type, and do something else in the case of an error, then deal with it at the point in the call stack it makes sense. The `shipcat::helm::parallel` module has some examples of this.

## Error-chain

Errors in shipcat are defined using the popular [error-chain crate](https://crates.io/crates/error-chain) ([docs](https://docs.rs/error-chain/)).

This makes it easy to incorporate foreign libraries `Error` types into our own `Error` type, by effectively creating a union of all the errors. This is what the `foreign_links` stuff is about.

But the key feature is that it allows you to `chain_err` and wrap errors into new errors by passing context up the stack.

As an example; this allows us to create our own error for templating, but still presenting the error from `tera` (the templating engine). Here is output from what happens when your template is invalid:

```
shipcat: validate error: Failed to render 'config.ini.j2'
shipcat: caused by: Variable `missing_var` not found in context while rendering 'config.ini.j2'
```

If we didn't chain errors, we'd have to keep contatenating error strings conditionally everywhere.

## Guidelines

### Use bail! to create root errors
Typically; this is a failure type you have defined. E.g.:

- manifests must have cpu resource limits set greater than or equal to requests
- service folder must contain your manifest

### Use question mark to propagate root errors
Typically; whenever you are using a library to do a more complicated operation that may have useful context. E.g.:

- http requests where we want to distinguish between 404 and 403
- serialization/deserialisation errors like where in the file the missing comma is

### Use chain_err to propagate root errors and add context
Same as above, but when you need additional context. We only actually do this in a few places because the error is usually sufficient from the root cause. A few examples:

- manifest validation errors that needs which service failed
- template errors that need which service failed

A common pattern so far is that `shipcat::cluster` benefits from adding which service failed as context.
