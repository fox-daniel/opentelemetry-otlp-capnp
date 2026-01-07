# Example of the Cap'n Proto SpanExporter handling bulk spans

CAREFUL: Until the startup logic is improved, the `receiver` has to be started before the `app`.

To run the example, in two separate terminals, run these commands from the `src` directory:
```
$ cargo run receiver
```
and
```
$ cargo run app
```
