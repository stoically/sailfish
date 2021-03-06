# Filters

Filters are used to format the rendered contents.

Example:

```ejs
message: <%= "foo\nbar" | dbg %>
```

Output:

```html
message: &quot;foo\nbar&quot;
```

!!! Note
    Since `dbg` filter accepts '<T: std::fmt::Debug>' types, that type isn't required to implement [`Render`](https://docs.rs/sailfish/latest/sailfish/runtime/trait.Render.html) trait. That means you can pass the type which doen't implement `Render` trait.


## Syntax

- Apply filter and HTML escaping

```ejs
<%= expression | filter %>
```

- Apply filter only

```ejs
<%- expression | filter %>
```

## Built-In Filters

Built-In filters can be found in [`sailfish::runtime::filter`](https://docs.rs/sailfish/latest/sailfish/runtime/filter/index.html) module.
