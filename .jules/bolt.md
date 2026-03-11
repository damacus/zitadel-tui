## 2024-05-18 - Avoid redundant TCP/SSL handshakes by reusing HTTP connections
**Learning:** Instantiating a new `Net::HTTP` connection for every API request results in redundant TCP connections and SSL handshakes, significantly slowing down performance when calling the same host (Zitadel API) repeatedly.
**Action:** Always consider using a persistent HTTP connection (`Net::HTTP.start`) or memoizing the connection object to reuse it for multiple API calls within the same client session.

## 2025-01-22 - Batch kubectl calls to avoid process spawning bottlenecks
**Learning:** Spawning external processes (like `kubectl`) is a significant bottleneck. Fetching individual secret keys with multiple `kubectl ... jsonpath` calls incurs a heavy process and network API overhead.
**Action:** Batch external calls to reduce overhead. When retrieving multiple values from a `kubectl` secret, use `kubectl ... -o json`, load the full JSON representation, and extract keys safely using `JSON.parse` rather than multiple `jsonpath` string parsing calls.
