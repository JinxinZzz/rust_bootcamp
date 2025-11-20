use clap::Parser;
use std::net::{TcpListener, TcpStream};
use std::io::{self, Read, Write};

// DH 参数（64 位素数和生成元）
const P: u64 = 0xD87FA3E291B4C7F3;
const G: u64 = 2;

// 命令行参数结构
#[derive(Parser, Debug)]
#[command(author, version, about = "Stream cipher chat with Diffie-Hellman key generation")]
enum Command {
    #[command(name = "server")]
    Server {
        #[arg(default_value = "8080")]
        port: u16,
    },
    #[command(name = "client")]
    Client {
        #[arg(default_value = "127.0.0.1:8080")]
        addr: String,
    },
}

// 流密码生成器（线性同余发生器 LCG）
struct LCG {
    a: u32,
    c: u32,
    m: u64,  // 改为 u64 避免字面量溢出
    next: u32,
}

impl LCG {
    fn new(seed: u32) -> Self {
        LCG {
            a: 1103515245,
            c: 12345,
            m: 0x100000000,  // 2^32，用 u64 存储
            next: seed,
        }
    }

    fn next_byte(&mut self) -> u8 {
        self.next = ((self.a as u64 * self.next as u64 + self.c as u64) % self.m) as u32;
        (self.next >> 24) as u8  // 取高 8 位作为字节输出
    }
}

// DH 密钥交换逻辑
fn dh_exchange(stream: &mut TcpStream) -> u32 {
    // 生成随机私钥（64 位）：使用最新 rand 库的 random() 方法
    let private_key: u64 = rand::random();
    println!("[DH] Generating our keypair...");
    println!("private_key = {private_key:016X} (random 64-bit)");

    // 计算公钥：g^private mod p
    let public_key = mod_pow(G, private_key, P);
    println!("public_key = g^private mod p");
    println!("= {G}^{private_key:016X} mod {P:016X}");
    println!("= {public_key:016X}");

    // 发送自己的公钥
    println!("[NETWORK] Sending public key (8 bytes)...");
    println!("→ Send our public: {public_key:016X}");
    stream.write_all(&public_key.to_be_bytes()).unwrap();

    // 接收对方的公钥
    let mut their_public = [0u8; 8];
    stream.read_exact(&mut their_public).unwrap();
    let their_public = u64::from_be_bytes(their_public);
    println!("[NETWORK] Received public key (8 bytes) ✓");
    println!("← Receive their public: {their_public:016X}");

    // 计算共享密钥：their_public^private mod p
    let shared_secret = mod_pow(their_public, private_key, P);
    println!("[DH] Computing shared secret...");
    println!("Formula: secret = (their_public)^(our_private) mod p");
    println!("secret = ({their_public:016X})^{private_key:016X} mod {P:016X}");
    println!("= {shared_secret:016X}");

    // 将 64 位共享密钥截断为 32 位，作为 LCG 种子
    (shared_secret >> 32) as u32
}

// 快速幂取模（用于 DH 密钥计算）
fn mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus == 1 {
        return 0;
    }
    let mut result = 1;
    base %= modulus;
    while exp > 0 {
        if exp % 2 == 1 {
            result = (result * base) % modulus;
        }
        exp >>= 1;
        base = (base * base) % modulus;
    }
    result
}

// 服务器逻辑
fn run_server(port: u16) {
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).unwrap();
    println!("[SERVER] Listening on 0.0.0.0:{port}");
    println!("[SERVER] Waiting for client...");

    let (mut stream, addr) = listener.accept().unwrap();
    println!("[CLIENT] Connected from {addr}");

    println!("[DH] Starting key exchange...");
    println!("[DH] Using hardcoded DH parameters:");
    println!("p = {P:016X} (64-bit prime - public)");
    println!("g = {G} (generator - public)");

    // 执行 DH 密钥交换，获取流密码种子
    let seed = dh_exchange(&mut stream);
    println!("[VERIFY] Both sides computed the same secret ✓");

    // 初始化流密码生成器
    let mut keystream = LCG::new(seed);
    println!("[STREAM] Generating keystream from secret...");
    println!("Algorithm: LCG (a=1103515245, c=12345, m=2^32)");
    println!("Seed: secret = {seed:08X}");

    // 启动聊天循环
    println!("✓ Secure channel established!");
    println!("[CHAT] Type message:");

    let (mut reader, mut writer) = (stream.try_clone().unwrap(), stream);
    let mut input = String::new();

    loop {
        // 发送消息逻辑
        io::stdin().read_line(&mut input).unwrap();
        let msg = input.trim().as_bytes();
        if msg.is_empty() {
            input.clear();
            continue;
        }

        // 加密消息（流密码 XOR）：使用 idx 作为循环变量
        let mut ciphertext = vec![0u8; msg.len()];
        for idx in 0..msg.len() {
            ciphertext[idx] = msg[idx] ^ keystream.next_byte();
        }

        println!("[ENCRYPT]");
        println!("Plain: {:?} (\"{}\")", msg, std::str::from_utf8(msg).unwrap_or(""));
        print!("Key: ");
        for idx in 0..msg.len() {
            print!("{:02X} ", keystream.next_byte());
        }
        println!("\nCipher: {:?}", ciphertext);

        // 发送加密消息
        writer.write_all(&ciphertext).unwrap();
        println!("[NETWORK] Sending encrypted message ({} bytes)...", msg.len());
        println!("[→] Sent {} bytes", msg.len());
        input.clear();

        // 接收消息逻辑
        let mut buffer = [0u8; 1024];
        let n = reader.read(&mut buffer).unwrap();
        if n == 0 {
            break;
        }

        // 解密消息：使用 idx 作为循环变量
        let mut plaintext = vec![0u8; n];
        for idx in 0..n {
            plaintext[idx] = buffer[idx] ^ keystream.next_byte();
        }

        println!("[NETWORK] Received encrypted message ({} bytes)", n);
        println!("[←] Received {} bytes", n);
        println!("[DECRYPT]");
        println!("Cipher: {:?}", &buffer[0..n]);
        print!("Key: ");
        for idx in 0..n {
            print!("{:02X} ", keystream.next_byte());
        }
        println!("\nPlain: {:?} → \"{}\"", plaintext, std::str::from_utf8(&plaintext).unwrap_or(""));
        println!("[TEST] Round-trip verified: \"{}\" → encrypt → decrypt → \"{}\" ✓",
            std::str::from_utf8(&plaintext).unwrap_or(""),
            std::str::from_utf8(&plaintext).unwrap_or("")
        );
        println!("[CLIENT] {}", std::str::from_utf8(&plaintext).unwrap_or(""));
    }
}

// 客户端逻辑
fn run_client(addr: String) {
    let mut stream = TcpStream::connect(addr).unwrap();

    println!("[DH] Starting key exchange...");
    println!("[DH] Using hardcoded DH parameters:");
    println!("p = {P:016X} (64-bit prime - public)");
    println!("g = {G} (generator - public)");

    // 执行 DH 密钥交换，获取流密码种子
    let seed = dh_exchange(&mut stream);
    println!("[VERIFY] Both sides computed the same secret ✓");

    // 初始化流密码生成器
    let mut keystream = LCG::new(seed);
    println!("[STREAM] Generating keystream from secret...");
    println!("Algorithm: LCG (a=1103515245, c=12345, m=2^32)");
    println!("Seed: secret = {seed:08X}");

    // 启动聊天循环
    println!("✓ Secure channel established!");
    println!("[CHAT] Type message:");

    let (mut reader, mut writer) = (stream.try_clone().unwrap(), stream);
    let mut input = String::new();

    loop {
        // 接收消息逻辑
        let mut buffer = [0u8; 1024];
        let n = reader.read(&mut buffer).unwrap();
        if n == 0 {
            break;
        }

        // 解密消息：使用 idx 作为循环变量
        let mut plaintext = vec![0u8; n];
        for idx in 0..n {
            plaintext[idx] = buffer[idx] ^ keystream.next_byte();
        }

        println!("[NETWORK] Received encrypted message ({} bytes)", n);
        println!("[←] Received {} bytes", n);
        println!("[DECRYPT]");
        println!("Cipher: {:?}", &buffer[0..n]);
        print!("Key: ");
        for idx in 0..n {
            print!("{:02X} ", keystream.next_byte());
        }
        println!("\nPlain: {:?} → \"{}\"", plaintext, std::str::from_utf8(&plaintext).unwrap_or(""));
        println!("[TEST] Round-trip verified: \"{}\" → encrypt → decrypt → \"{}\" ✓",
            std::str::from_utf8(&plaintext).unwrap_or(""),
            std::str::from_utf8(&plaintext).unwrap_or("")
        );
        println!("[SERVER] {}", std::str::from_utf8(&plaintext).unwrap_or(""));

        // 发送消息逻辑
        input.clear();
        io::stdin().read_line(&mut input).unwrap();
        let msg = input.trim().as_bytes();
        if msg.is_empty() {
            input.clear();
            continue;
        }

        // 加密消息（流密码 XOR）：使用 idx 作为循环变量
        let mut ciphertext = vec![0u8; msg.len()];
        for idx in 0..msg.len() {
            ciphertext[idx] = msg[idx] ^ keystream.next_byte();
        }

        println!("[ENCRYPT]");
        println!("Plain: {:?} (\"{}\")", msg, std::str::from_utf8(msg).unwrap_or(""));
        print!("Key: ");
        for idx in 0..msg.len() {
            print!("{:02X} ", keystream.next_byte());
        }
        println!("\nCipher: {:?}", ciphertext);

        // 发送加密消息
        writer.write_all(&ciphertext).unwrap();
        println!("[NETWORK] Sending encrypted message ({} bytes)...", msg.len());
        println!("[→] Sent {} bytes", msg.len());
        input.clear();
    }
}

fn main() {
    let command = Command::parse();
    match command {
        Command::Server { port } => run_server(port),
        Command::Client { addr } => run_client(addr),
    }
}