# Zelyra password generator benchmark

This is a protected, server-rendered Zelyra application. Its sign-in, sessions,
roles, account administration, password form, validation, and random generation
are composed from native Zelyra declarations and reusable local packages.

## Run

The project `.env` selects `ZELYRA_LEVEL=work` and German (`de`) UI. The
application also needs an isolated MariaDB database for users, hashed
credentials, roles, audit records, and sessions. Configure `DATABASE_URL`, then
run:

```sh
zelyra package lock main.zyl
zelyra db setup main.zyl
zelyra check main.zyl
zelyra run tests.zyl
zelyra serve main.zyl
```

Open `/login` to sign in. Self-service registration is deliberately not
published. An authorized administrator creates and manages accounts at
`/admin/users`; grant the `admin` role the `identity.users.manage` and
`password_generator.use` permissions during initial provisioning.

The app is composed from three local packages:

- `zelyra_identity`: reusable Zelyra authentication, persistent sessions,
  roles, audit records, and protected user administration.
- `password_app`: protected web pages and the typed password form handler.
- `password_engine`: password generation logic.

The package system is still experimental; this is a local-package integration
example, not a published registry package or a stable plugin ABI.

The application theme is regular project CSS in `zelyra.theme.css` using the
native Zelyra UI tokens and components. It provides a branded, responsive sign-in
screen and a matching generator surface. The environment chip is hidden from
the product UI.

## Generator behavior

The generator accepts lengths from 8 to 64 and provides lowercase-only,
digit-only, letters, letters plus digits, and all-character profiles.
`random_int` uses Zelyra's explicit `Random` capability. Independent secure draws
and rejection sampling ensure every selected character class appears without
biasing the accepted output. Generated values are escaped, returned with
`Cache-Control: no-store, private`, and never stored or placed in a URL.

The standalone handler action uses Zelyra's server-side authentication and
permission checks before the function can run. The form also enforces CSRF
protection and typed server-side validation.
