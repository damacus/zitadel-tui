## 2024-05-18 - Avoid redundant TCP/SSL handshakes by reusing HTTP connections
**Learning:** Instantiating a new `Net::HTTP` connection for every API request results in redundant TCP connections and SSL handshakes, significantly slowing down performance when calling the same host (Zitadel API) repeatedly.
**Action:** Always consider using a persistent HTTP connection (`Net::HTTP.start`) or memoizing the connection object to reuse it for multiple API calls within the same client session.

## 2024-05-18 - Avoid redundant process calls for Kubernetes secrets
**Learning:** Calling `kubectl` to fetch individual keys from the same Kubernetes secret introduces significant overhead due to process spawning, configuration loading, and duplicate network requests to the Kubernetes API.
**Action:** Always batch external process calls where possible. For `kubectl`, use `jsonpath` (e.g., `-o jsonpath={.data.key1} {.data.key2}`) to extract multiple fields in a single execution.
