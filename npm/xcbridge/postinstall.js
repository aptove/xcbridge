/**
 * xcbridge postinstall script
 * 
 * Verifies the correct platform-specific package was installed
 * and the binary is executable.
 */

const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const PLATFORM_PACKAGES = {
  'darwin-arm64': '@aptove/xcbridge-darwin-arm64',
  'darwin-x64': '@aptove/xcbridge-darwin-x64',
};

function main() {
  const platformKey = `${process.platform}-${process.arch}`;
  const packageName = PLATFORM_PACKAGES[platformKey];

  if (!packageName) {
    console.warn(`⚠️  xcbridge: Unsupported platform ${platformKey}`);
    console.warn('   xcbridge only supports macOS (darwin-arm64, darwin-x64)');
    return;
  }

  // Check if platform package exists
  try {
    const packagePath = require.resolve(`${packageName}/package.json`);
    const binaryPath = path.join(path.dirname(packagePath), 'bin', 'xcbridge');
    
    if (!fs.existsSync(binaryPath)) {
      console.warn(`⚠️  xcbridge: Binary not found at ${binaryPath}`);
      return;
    }

    // Ensure binary is executable
    try {
      fs.chmodSync(binaryPath, 0o755);
    } catch (e) {
      // Might fail on some systems, not critical
    }

    // Verify binary works
    try {
      execSync(`"${binaryPath}" --version`, { stdio: 'pipe' });
      console.log(`✓ xcbridge installed successfully for ${platformKey}`);
    } catch (e) {
      console.warn(`⚠️  xcbridge: Binary exists but failed to execute`);
      console.warn(`   This might be due to missing Xcode installation`);
    }
  } catch (e) {
    // Optional dependency not installed - this is expected on CI or unsupported platforms
    console.warn(`⚠️  xcbridge: Platform package ${packageName} not installed`);
    console.warn(`   This is expected if you're not on ${platformKey}`);
  }
}

main();
