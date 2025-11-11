| bench | parameter | samples | min (ns) | mean (ns) | stddev (ns) |
| --- | --- | ---: | ---: | ---: | ---: |
| mahler_next | default | 50 | 331295.965 | 334469.848 | 4694.830 |
| mahler_regen | default | 50 | 332862.857 | 342867.590 | 16834.185 |
| random_faces_next | 5-10 | 50 | 23092.944 | 23395.629 | 217.184 |
| random_vertices_next | 5-25 | 50 | 6711512.667 | 6848061.177 | 171855.630 |
| regular_product_next | 8x10 | 50 | 726.073 | 735.122 | 10.641 |
| regular_product_regen | 8x10 | 50 | 693.845 | 724.478 | 15.939 |
| sym_halfspaces_generate_single | d5 | 50 | 23405.376 | 23791.267 | 664.879 |

_Updated 2025-11-11 01:41:11Z · commit 585a129 · host ab5b4864ef14 · rustc rustc 1.91.1 (ed61e7d7e 2025-11-07)_
