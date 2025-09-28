from pathlib import Path
import sys

path = Path('space-downloader-core/src/download.rs')
text = path.read_text()
old = "        let mut state = self.inner.state.lock().await;\n        state.max_concurrency = self.inner.config.read().await.download.effective_concurrency();\n        drop(state);\n        schedule_jobs(self.inner.clone()).await;\n"
new = "        let concurrency = self\n            .inner\n            .config\n            .read()\n            .await\n            .download\n            .effective_concurrency();\n        let mut semaphore = self.inner.semaphore.write().await;\n        *semaphore = Arc::new(Semaphore::new(concurrency));\n"
if old not in text:
    sys.exit('update_config block not found')
path.write_text(text.replace(old, new, 1))
