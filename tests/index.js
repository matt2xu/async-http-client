const app = require('express')();

app.get('/', (req, res) => {
    res.set('Connection', 'close');
    res.send('hello, world!');
});

app.post('/post-test', (req, res) => {
    res.status(204).end();
});

app.listen(3000, () => {
    console.log('Test server running on port 3000!')
});
