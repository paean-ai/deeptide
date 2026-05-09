# Three Showcase

A single entry page for the Three.js examples.

## Included Samples

- `examples/neon-orbits`
- `examples/lowpoly-island`
- `examples/product-configurator`
- `shared` vendored Three.js module

## Run

Serve the `samples/three-showcase` folder:

```bash
cd samples/three-showcase
python3 -m http.server 4192
```

Then open:

```text
http://127.0.0.1:4192/index.html
```

## Publish

For a single deployable Three.js showcase bundle, publish this folder as-is:

```text
three-showcase/
  index.html
  css/
  js/
  examples/
    neon-orbits/
    lowpoly-island/
    product-configurator/
  shared/
    three.module.js
```

The showcase uses relative iframe paths, and each sample imports `../../../shared/three.module.js`.
