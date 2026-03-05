## 2024-05-18 - Avoid redundant TCP/SSL handshakes by reusing HTTP connections
**Learning:** Instantiating a new `Net::HTTP` connection for every API request results in redundant TCP connections and SSL handshakes, significantly slowing down performance when calling the same host (Zitadel API) repeatedly.
**Action:** Always consider using a persistent HTTP connection (`Net::HTTP.start`) or memoizing the connection object to reuse it for multiple API calls within the same client session.
