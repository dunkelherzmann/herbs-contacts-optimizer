# Data and fonts from others

- Country codes and dialling prefixes (`core/src/countries_data.rs`) come from libphonenumber's
  `PhoneNumberMetadata.xml`, © Google and contributors, Apache License 2.0
  (github.com/google/libphonenumber).
- Country names in the same file come from Unicode CLDR, © Unicode, Inc., Unicode License v3
  (github.com/unicode-org/cldr-json).

`scripts/countries.py` makes the file from the two sources.

- The Linux and Windows versions carry the Inter font (`app/assets/Inter-Regular.ttf`), © The Inter Project
  Authors, SIL Open Font License 1.1 (github.com/rsms/inter; the licence is `app/assets/Inter-LICENSE.txt`).
