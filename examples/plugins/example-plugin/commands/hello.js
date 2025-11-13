#!/usr/bin/env node

/**
 * Hello Command
 *
 * Simple command that greets the user by name.
 */

const args = JSON.parse(process.argv[2] || '{}');

if (!args.name) {
  console.error('Error: name argument is required');
  process.exit(1);
}

console.log(`Hello, ${args.name}! Welcome to the Claude Code plugin system.`);
console.log(`This is an example command showing how plugins work.`);

process.exit(0);
