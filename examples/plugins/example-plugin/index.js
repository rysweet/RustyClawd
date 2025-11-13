#!/usr/bin/env node

/**
 * Example Plugin Entry Point
 *
 * This file serves as the main entry point for the plugin.
 * It can initialize resources, set up configuration, etc.
 */

console.log('Example plugin loaded successfully!');

// Export plugin metadata if needed
module.exports = {
  name: 'example-plugin',
  version: '1.0.0',
  initialized: true
};
