# Zelyra password generator benchmark

This small web application is the first practical acceptance project after the
Zelyra 1.3.0 release. Its form, validation, random generation, and response are
implemented in Zelyra. It uses no JavaScript application code, database,
external service, or persisted password values.

## Run

The project `.env` selects `ZELYRA_LEVEL=work` and German (`de`) UI. From this
directory, run:

```sh
zelyra check main.zyl
zelyra run tests.zyl
zelyra serve main.zyl
```

Open the local address printed by the server. The generator accepts lengths
from 8 to 64 and provides lowercase-only, digit-only, letters, letters plus
digits, and all-character profiles.

## Security behavior

- `random_int` uses Zelyra's explicit `Random` capability and secure inclusive
  integer ranges.
- Independent secure draws and rejection sampling ensure each returned value
  contains every character class required by its selected profile.
- The standalone form enforces CSRF protection and server-side validation.
- Results are HTML-escaped and returned with `Cache-Control: no-store,
  private`. They are not written to a database or included in a URL.

This benchmark also exercises native standalone form handlers: a declared
Zelyra function receives typed, validated fields and returns a `String` result
for safe rendering.
