Module that provides object persistence

The `rio_rs::state` module has all the traits and structs one needs
to start saving objects from your cluster to persistent storage.

# State

Trait for implementing how to deserialize data in, and serialize data out of
a service (trait object).

Although you need to implement this trait for each state you have in your application,
the `ManagedState` derive implements it for you.

# State Loader

Any type that implements `rio_rs::state_provider::StateLoader`.

The `StateLoader` defines an interface to load state data from a backend.

# State Saver

Any type that implements `rio_rs::state_provider::StateSaver`.

It defines an interface to save data into a persistence backend.

# State Provider

Any type that is both a `State Saver` and a `State Loader`.

It provides functions to load and deserialize state, and serialize and save state.
