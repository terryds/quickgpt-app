# QuickGPT App

This is a Tauri-based Windows app powered by GPT-4, providing a quicker alternative to ChatGPT Plus

# How to install

1. Deploy the frontend
This repository is only for the desktop app (Tauri) and behind the scene it loads a webview from a URL (the frontend), so we need to go to the frontend repo and deploy it
Frontend Repo: https://github.com/terryds/quickgpt-ui

2. Change the URL variable in src/tauri/src/main.rs to the deployed url
```
#[cfg(not(dev))]
const URL: &str = "";
```

3. Build the app
```
npm run tauri build
```

4. Use the .msi installer bundle to install the app

## Credits

- Chatbot UI by McKayWrigley ( https://github.com/mckaywrigley/chatbot-ui )
- Chat AI Desktop by Sonny Lazuardi ( https://github.com/sonnylazuardi/chat-ai-desktop )

## Other Contributors
- MrAdhit ( https://github.com/MrAdhit )