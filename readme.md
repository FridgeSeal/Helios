# Tarkine


## About 
An experiment in percolation-query full-text-search with thread-per-core architecture.

Percolation queries are an inversion of the normal full-text search style query. In the mormal case, the system indexes and persists *documents* in order to quickly answer ephemeral queries. With percolation (also called 'document routing' or 'reverse text search') style search, we instead persist the *queries* (and matching results) and stream documents through the engine. This makes an interesting trade-off in that we give up the ability to respond to "ad-hoc" queries quickly, but gain the ability to have persistent asynchronous search results.

Please note - this is very much a work-in-progress, largely to explore the design space and improve my programming skills.

### Design and motivation
