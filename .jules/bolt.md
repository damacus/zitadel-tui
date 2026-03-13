## 2024-05-18 - Avoid redundant TCP/SSL handshakes by reusing HTTP connections
**Learning:** Instantiating a new `Net::HTTP` connection for every API request results in redundant TCP connections and SSL handshakes, significantly slowing down performance when calling the same host (Zitadel API) repeatedly.
**Action:** Always consider using a persistent HTTP connection (`Net::HTTP.start`) or memoizing the connection object to reuse it for multiple API calls within the same client session.
## 2026-03-12 - Batch kubectl secret retrievals to avoid process overhead
**Learning:** Spawning external processes like `kubectl` is a significant bottleneck. Fetching individual secret keys with multiple `kubectl` calls multiplies process spawning and network API overhead.
**Action:** Always batch external tool calls when possible. When retrieving secrets, use `kubectl ... -o json` and parse the output instead of executing `kubectl` for each key individually.
