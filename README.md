_Multi-party computation experimentation library_

This library simulates multiple parties who collaborate in a multi-party computation protocol.
The simulator assigns each party a single thread and mimics communication channels through asynchronous channels.
The number of bytes that each party transfers is tracked, and additional features such as timing allows a developer to
get a clear view of performance by examining the resulting statistics after evaluting the protocol.

To implement a protocol, one must implement the `Protocol<I,O>` trait for a custom struct. `I` is the input type and `O` is the output type.
The important method here is `fn run_party(id: usize, n_parties: usize, this_party: Party, input: I) -> (PartyStats, O);`,
which contains the code for an individual party to run. The party learns its unique `id`, the number of total parties `n_parties`,
and its `input`. The `this_party` object offers functionality for sending and receiving messages, among others.
Next to this party's protocol output, this method must output the party's statistics, which can be done using `this_party.get_stats()`.

After `run_party` is implemented, a developer can evaluate the protocol for a given number of parties and respective inputs
by calling `evaluate(5, vec![10; 5])`, for example. In this case that would spawn five parties who each have the input `10`.
