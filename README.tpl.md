# {{crate}}

![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/rcelha/rio-rs/.github%2Fworkflows%2Frust.yaml?style=for-the-badge)
![Crates.io Version](https://img.shields.io/crates/v/rio-rs?style=for-the-badge&link=https%3A%2F%2Fcrates.io%2Fcrates%2Frio-rs)
![docs.rs](https://img.shields.io/docsrs/rio-rs?style=for-the-badge&link=https%3A%2F%2Fdocs.rs%2Frio-rs%2Flatest%2Frio_rs%2F)

---

{{readme}}

---

## Features

Here are some of the features that are fully implemented:

### Clustering

Clustering is divided in two parts: `Membership Protocol` and `Membership Storage`.

The Membership Storage is responsible for the rendezvous of the cluster, it manages which nodes are members of the
clusters, and how to store the nodes' state in the cluster. Both server and client need to have access to the Membership Storage.

The Membership Protocol is a server that run in each node of the cluster, it is reponsible for testing
the nodes to define which nodes are alive and which are dead.
The Memebership Protocols utilize the Membership Storage to store the state of the nodes in the cluster.

Currently, we only have a `PeerToPeerClusterProvider`, which is a simple implementation of the cluster membership protocol that uses a gossip protocol to keep track of the nodes in the cluster.

As for Storages, we have a few:

- LocalStorage: A simple in-memory storage, built just for testing
- HttpMembershipStorage: A read-only storage that uses HTTP API to expose information of the cluster, it is useful to use this on the client side, but it should never be used on the server side, since it is read-only and the server needs to update the state of the cluster.
- PostgresMembershipStorage
- RedisMembershipStorage
- SqliteMembershipStorage

### Object Placement

Object Placement maps each object's location in the cluster. Only the server has access to the Object Placement, and it is used by the server to know where to send the requests for each object.

- LocalObjectPlacement: Simple in-memory object placement, built just for testing
- PostgresObjectPlacement
- RedisObjectPlacement
- SqliteObjectPlacement

### Object Persistence (Managed State)

Rio offers a way to manage the state of your objects in a persistent storage.
You can simply drop the `ManagedState` derive on your struct, and it will automatically implement necessary
traits to serialize and deserialize your struct, and to save and load it from a persistence backend.
Alternatively, you can implement the persistence traits manually, if you need more control over how your state is saved and loaded.

Here are the built-in persistence backends:

- LocalState: A simple in-memory state, built just for testing
- PostgresState
- RedisState
- SqliteState
