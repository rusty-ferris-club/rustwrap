const { platform, arch } = process;

const INFO = require('./info.json');
const name = INFO.name;

const binName = INFO.platforms.find((p) => p.platform == platform && p.arch == arch)?.bin;
if (binName) {
	let binPath;
	try {
		binPath = require.resolve(binName);
	} catch {
		console.warn(
			`The ${name} CLI postinstall script failed to resolve the binary file "${binName}". Running ${name} from the npm package will probably not work correctly.`,
		);
	}

	if (binPath) {
		try {
			require("fs").chmodSync(binPath, 0o755);
		} catch {
			console.warn(
				`The ${name} CLI postinstall script failed to set execution permissions to the native binary. ` +
					`Running ${name} from the npm package will probably not work correctly.`,
			);
		}
	}
} else {
	console.warn(
		`The ${name} CLI package doesn't ship with prebuilt binaries for your platform yet.`
	);
}
