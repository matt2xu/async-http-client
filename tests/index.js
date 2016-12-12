const app = require('express')();

app.get('/', (req, res) => {
    res.end('hello, world!');
});

app.post('/post-test', (req, res) => {
    res.status(204).end();
});

app.listen(3000, () => {
    console.log('Test server running on port 3000!')
});
