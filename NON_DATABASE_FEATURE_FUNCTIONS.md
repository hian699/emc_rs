# Danh sach function tinh nang khong su dung database

## Tieu chi loc
- Chi lay function phuc vu tinh nang chay that trong `commands`, `components`, `events`, `utils`.
- Loai bo boilerplate (`structure/*`, handler, file `example-*`, wiring khoi tao chung).
- Loai bo file co truy cap DB truc tiep (`Models/*`, `mongoose`, `find/findOne/save/update/delete`) hoac phu thuoc util DB.

## Commands (feature handlers)
- `src/commands/Utility/slashcommand-ping.js` -> `run`
- `src/commands/Utility/messagecommand-ping.js` -> `run`
- `src/commands/User/slashcommand-random.js` -> `run`
- `src/commands/Administrator/slashcommand-timeout.js` -> `run`
- `src/commands/Administrator/slashcommand-deletemessage.js` -> `run`
- `src/commands/Administrator/slashcommand-security-lockdown.js` -> `setLockdown`, `run`
- `src/commands/Developer/slashcommand-reload.js` -> `run`
- `src/commands/Developer/messagecommand-reload.js` -> `run`
- `src/commands/Developer/slashcommand-eval.js` -> `run`
- `src/commands/Developer/messagecommand-eval.js` -> `run`

## Components (interactive features)
- `src/components/Button/music-skip.js` -> `run`
- `src/components/Button/music-stop.js` -> `run`
- `src/components/Button/music-clear.js` -> `run`
- `src/components/SelectMenu/music-search.js` -> `run`, `formatDuration`
- `src/components/SelectMenu/private-voice-invite.js` -> `run`

## Events (runtime feature hooks)
- `src/events/VoiceStateUpdate/onMusicAutoLeave.js` -> `run`

## Utils (feature logic)
- `src/utils/MusicQueue.js` -> `constructor`, `connect`, `addSong`, `play`, `handleSongEnd`, `skip`, `stop`, `destroy`, `sendError`, `startDisconnectTimeout`, `clearDisconnectTimeout`, `getQueueInfo`
- `src/utils/MusicManager.js` -> `constructor`, `getQueue`, `hasQueue`, `createQueue`, `deleteQueue`, `getAllQueues`
- `src/utils/YtDlpHelper.js` -> `getCommand`, `execute`, `getVideoInfo`, `search`
- `src/utils/SearchCache.js` -> `constructor`, `storeResults`, `get`, `has`, `cleanup`, `clear`
- `src/utils/CookieManager.js` -> `getCookiePath`, `isValidCookieFile`, `checkAndCreate`, `getArgs`, `logStatus`
- `src/utils/PrivateVoiceRegistry.js` -> `setOwner`, `getOwner`, `deleteOwner`
- `src/utils/passwordEncryptor.js` -> `hashPassword`, `comparePassword`
- `src/utils/milisecondCalculator.js` -> `(module export function)`
- `src/utils/LevelCalculator.js` -> `(module export function)`

## Ghi chu
- Danh sach tren uu tien cac tinh nang van hanh truc tiep (music, moderation command, private voice, utility logic).
- Da loai cac file lien quan DB ro rang nhu: `signIn-modal`, `usercontext-userinfo`, `onVoiceState`, cac module task/shop/credit/event clock va cac event logging dung `Config`.
