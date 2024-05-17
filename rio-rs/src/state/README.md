Module that provides object persistence

The `rio_rs::state` module has all the traits and structs one needs
to start saving objects from your cluster to persistent storage.

# State Loader

Any type that implements `rio_rs::state_provider::StateLoader`

# State Saver

Any type that implements `rio_rs::state_provider::StateSaver`

# State Provider

Any type that is both a `State Saver` and a `State Loader`
