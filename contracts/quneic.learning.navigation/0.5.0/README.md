# Learning navigation 0.5.0

Navigation.Publish returns a core Posit containing the source Chapter’s Tablet Actions. The receiving Frontier controls admission. Each Action carries the exact destination invocation selected by the owner. Executing that invocation requires a separately issued grant.

The owner pins the source course, map and scratch selections, the destination editions and Chapter selections, and the Posit identity. It derives producer_cut from its own method binding and supplies only pre-authorized destination invocations. The guest matches them against the requests it derives from the selected map and publishes a computed Posit attributed to that cut.

The standalone navigation.fds imports the existing quneic.learning.contract 0.5.0 descriptor. To reproduce the descriptor from this directory:

```sh
protoc -I. --descriptor_set_in=../../quneic.learning/0.5.0/learning.fds \
  --descriptor_set_out=navigation.fds quneic/learning/navigation/v05/navigation.proto
shasum -a 256 navigation.fds
```

A passage publishes its declared exercise destinations. At an exercise, the owner may include one or more related reading editions in `destinations`. For each such edition the publisher follows placements whose `exercise` equals the current CellRef, verifies `at` against the pinned reading order, and uses that Chapter's title. Each produced Action requires its own exact destination invocation. Omitting the related editions leaves them out of the publication. Both directions preserve the requested map and scratch cuts.
