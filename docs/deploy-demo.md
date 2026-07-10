# Public demo deployment (Clever Cloud, static)

The demo is the web UI in **local mode only**: pure static files + WASM, no
server component, no data leaves the visitor's browser. It embeds an
illustrative catalogue with **zero Artificial Analysis data** (their free
tier is internal-use-only) — benchmark columns stay empty by design, with the
attribution banner always visible.

## Build

```sh
scripts/build-demo.sh        # → dist/demo/
```

## Deploy (first time)

```sh
clever create --type static-apache rumble-ai-clearance-demo
clever domain add clearance-demo.<your-domain>
```

Deploy the `dist/demo` content as the webroot. Two options:

- **Subtree push**: commit `dist/demo` on a `deploy` branch and
  `clever deploy --branch deploy` with `CC_WEBROOT=/dist/demo` set via
  `clever env set CC_WEBROOT /dist/demo`.
- **Dedicated deploy repo**: copy `dist/demo/*` into the Clever-linked repo
  and push (keeps the product repo free of build artifacts).

## Redeploy

```sh
scripts/build-demo.sh && clever deploy
```

## Checklist before going public

- [ ] `npx playwright test` green (3 browsers) on the exact bundle
- [ ] The attribution banner ("Artificial Analysis", "internal-use-only") is
      visible on the deployed page
- [ ] No `data/` snapshot or AA-derived numbers anywhere in `dist/demo`
      (`grep -ri artificial_analysis_ dist/demo` must only match the
      attribution strings, never index values)
- [ ] `Cache-Control` acceptable for WASM (Clever static defaults are fine)
