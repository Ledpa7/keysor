const fs = require('fs');
const path = require('path');

const src = path.join(__dirname, 'homepage', 'dist');
const dest = path.join(__dirname, 'public');

function copyRecursiveSync(src, dest) {
  const exists = fs.existsSync(src);
  const stats = exists && fs.statSync(src);
  const isDirectory = exists && stats.isDirectory();
  if (isDirectory) {
    if (!fs.existsSync(dest)) {
      fs.mkdirSync(dest, { recursive: true });
    }
    fs.readdirSync(src).forEach(function(childItemName) {
      copyRecursiveSync(path.join(src, childItemName), path.join(dest, childItemName));
    });
  } else {
    fs.copyFileSync(src, dest);
  }
}

try {
  copyRecursiveSync(src, dest);
  console.log('Successfully copied homepage/dist to public (Cross-platform support)!');
} catch (err) {
  console.error('Failed to copy files:', err);
  process.exit(1);
}
