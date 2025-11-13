#!/usr/bin/env node

/**
 * Code Analysis Command
 *
 * Analyzes a file for code quality issues.
 */

const fs = require('fs');
const args = JSON.parse(process.argv[2] || '{}');

if (!args.file) {
  console.error('Error: file argument is required');
  process.exit(1);
}

// Check if file exists
if (!fs.existsSync(args.file)) {
  console.error(`Error: File not found: ${args.file}`);
  process.exit(1);
}

// Read and analyze file
const content = fs.readFileSync(args.file, 'utf8');
const lines = content.split('\n');
const strict = args.strict || false;

console.log(`Analyzing: ${args.file}`);
console.log(`Mode: ${strict ? 'strict' : 'normal'}`);
console.log(`Lines: ${lines.length}`);
console.log(`Characters: ${content.length}`);

// Simple analysis
const issues = [];

if (content.includes('TODO')) {
  issues.push('Contains TODO comments');
}

if (content.includes('console.log') && strict) {
  issues.push('Contains console.log statements (strict mode)');
}

if (lines.length > 500 && strict) {
  issues.push('File is very long (>500 lines)');
}

if (issues.length > 0) {
  console.log('\nIssues found:');
  issues.forEach((issue, i) => {
    console.log(`  ${i + 1}. ${issue}`);
  });
} else {
  console.log('\nNo issues found!');
}

process.exit(0);
