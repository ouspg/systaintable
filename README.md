# Systaintable

Systaintable is a OUSPG research project aiming to discover potential vulnerabilities from system level event log and traces through input propagation analysis. This Systaintable project repository includes following tooling for doing the research project:

* OUSPG Logbook
    * tracking event trace material collection, facts, processing and removal over its whole life-cycle (“chain of custody”)
* OUSPG ID.ID
    * Identifying and classifying identities (natural or artificial) from the event trace material
    normalization of the event traces -> isolating the variables -> identifying identities from the variable values / classifying the identities into identity categories
* OUSPG ID.ALIAS
    * Identifying aliases of the same underlying “entities” from the identity values
    span over different abstraction levels (mac, ip, dns, url, …) (name, userid, public key,…)
* OUSPG ID.TRACE
    * getting vectors of ID occurrences (extracting the traces) (@ time, locus, component, …)

Additionally this project repository contains guides how to use other tools for doing system level taint analysis.
