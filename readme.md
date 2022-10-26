# Tarkine


## About 
An experiment in percolation-query full-text-search with thread-per-core architecture.

Percolation queries are an inversion of the normal full-text search style query. In the mormal case, the system indexes and persists *documents* in order to quickly answer ephemeral queries. With percolation (also called 'document routing' or 'reverse text search') style search, we instead persist the *queries* (and matching results) and stream documents through the engine. This makes an interesting trade-off in that we give up the ability to respond to "ad-hoc" queries quickly, but gain the ability to have persistent asynchronous search results.

Please note - this is very much a work-in-progress, largely to explore the design space, learn some new tools and technologies and improve my programming skills.

### Motivation
Full text search, and search engines sit at the nexus of a lot of fascinating areas of computer science, maths and high-performance software engineering. This means there's large "design space" of different tradeoffs and specialisations, which at least within the space of day-to-day software development appears to be relatively under-explored, especially when combined with modern CPU speeds and thread-counts, NVMe drives & io_uring and novel application architectures like compute-storage-separation out of cloud environments.

### Design
Here's some rudimentary ascii art of the intended architecture of V1
```
                          ┌──────────┐  ┌──────────┐
                          │          │  │          │
                          │          │  │ HTTP/2/3 │
                          │  Kafka   │  │   API    ◄──────┐
                          │ Consumer │  │ 1 Thread │      │
                          │          │  │          │      │
                   ┌──────┴─────┬────┴──┴────┬─────┴──────┼─────────────┐
                   │            │            │            │             │
          channels │            │            │            │             │
          to       │            │            │            │             │
          stream   │            │            │            │             │
          docs in  │            │            │            │             │
                   │            │            │            │             │
              ┌────▼─────┐ ┌────▼─────┐ ┌────▼─────┐ ┌────▼─────┐  ┌────▼─────┐
              │          │ │          │ │          │ │          │  │          │
queries       │ Search   │ │ Search   │ │ Search   │ │ Reader   │  │ External │
are sharded   │ Thread   │ │ Thread   │ │ Thread   │ │ Thread   │  │ Writer   │ External writer thread
by core       │  1/n     │ │  2/n     │ │  n/n     │ │          │  │ Thread   │ mirrors local state into
              │          │ │          │ │          │ │          │  │          │ downstream api/kafka/object-store
              └────┬─────┘ └────┬─────┘ └────┬─────┘ └────▲─────┘  └───▲─┬────┘ for durability + recovery                            
                   │            │            │            │            │ │
         io_uring  │            │            │            │  ┌─────────┘ │
                   │            │            │            │  │           │
              ┌────▼────────────▼────────────▼────────────▼──┴──┐   ┌────▼─────┐
              │                                                 │   │          │
              │        Local NVMe drive                         │   │  Object  │
              │                                                 │   │  Store/  │
              └─────────────────────────────────────────────────┘   │   etc    │
                                                                    │          │
                rkyv with an embedded kv-store for local writes     └──────────┘
                to enable zero-copy deserialisation + mutation
```