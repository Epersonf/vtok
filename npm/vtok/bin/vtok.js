#!/usr/bin/env node
const { runCli } = require('../index.js');
process.exitCode = runCli(process.argv.slice(2));
