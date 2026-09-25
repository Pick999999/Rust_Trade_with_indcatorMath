const fs = require('fs');

const setup = JSON.parse(fs.readFileSync('setup.json', 'utf8'));
const indicators = setup.indicators;

console.log('Indicators from setup.json:', indicators);

// Simulate what serde_json::from_str does for camelCase
const atrMulti = indicators.atrMulti;
console.log('ATR Multi:', atrMulti);
