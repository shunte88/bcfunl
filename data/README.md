# Country names

`countries.tsv` contains ISO 3166-1 alpha-2 codes and English names extracted
from iso-codes 4.20.1 (`/usr/share/iso-codes/json/iso_3166-1.json`).
Source: https://salsa.debian.org/iso-codes-team/iso-codes

Copyright 2001-2008 Alastair McKinstry, 2004-2016 Christian Perrier,
and 2005-2026 Dr. Tobias Quathamer. Licensed under LGPL-2.1-or-later;
see `COPYING.LIB`. The table is bundled so runtime lookup needs no installed
dataset or network service. Bandcamp aliases are handled in Rust.

# Genre transformation overrides

`genremap.dat` adds bcfunl-specific aliases to VCW's shared genre table. It uses
`source|target;target` rows, currently including descriptive EBM aliases. The
table is embedded in the executable; rebuild after editing it. Canonical short
acronyms from the mapping, such as EBM and IDM, retain uppercase spelling.
