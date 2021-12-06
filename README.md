Utility to find errors in a DSL file (dictionary format by Lingvo, used by GoldenDict as well).

Type of detected errors:

- invalid order of entities. E.g, a body must follow a keyword, so two keywords in a row is an error
- stray `[` and `]`. Lingvo compiler may fail on such "tags", GoldenDict just hides them. Use `fix-tags` command to escape all stray square brackets
- leading spaces instead of leading TABs
- mismatched opening and closing tags
