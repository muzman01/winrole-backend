// bot.js
const TelegramBot = require('node-telegram-bot-api');

// Bot tokeninizi buraya girin
const token = '7066575107:AAExt9HNG6kR7_vYy4O8CovqAyLIJsD3E3o'; // BotFather'dan aldığınız token

// Botu başlat ve polling (düzenli kontrol) modunda çalıştır
const bot = new TelegramBot(token, { polling: true });

// /start komutunu işleme
bot.onText(/\/start/, (msg) => {
  const chatId = msg.chat.id;
  bot.sendMessage(chatId, 'Merhaba! Botumuza hoş geldiniz. /help komutunu kullanarak neler yapabileceğinizi görebilirsiniz.');
});

// /help komutunu işleme
bot.onText(/\/help/, (msg) => {
  const chatId = msg.chat.id;
  bot.sendMessage(chatId, 'Yardım Menüsü:\n\n/start - Botu başlatır\n/help - Yardım bilgilerini gösterir\n/about - Bot hakkında bilgi verir');
});

// /about komutunu işleme
bot.onText(/\/about/, (msg) => {
  const chatId = msg.chat.id;
  bot.sendMessage(chatId, 'Bu bot, Node.js ve Telegram API kullanılarak oluşturulmuştur.');
});

// Diğer mesajları karşılama
bot.on('message', (msg) => {
  const chatId = msg.chat.id;
  if (!msg.text.startsWith('/')) { // Komut dışındaki mesajları karşılama
    bot.sendMessage(chatId, 'Merhaba! Size nasıl yardımcı olabilirim? Komutlar için /help yazabilirsiniz.');
  }
});

console.log('Bot çalışıyor...');
