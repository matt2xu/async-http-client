const http = require('http');

const server = http.createServer((req, res) => {
  res.statusCode = 204;
  res.end();
});
server.listen(3000);
