#!/usr/bin/env php
<?php
/*************************
COPYRIGHT LESTER COVEY (me@lestercovey.ml) 
AND MOROZOVSK (https://habr.com/ru/post/209864/),
2022

*************************/

// TODO:
// All this party only works on `ws` (unsafe websocket).
// Though there's some additional encryption going on, 
// I'd like to make this thing capable of handling `wss` (with SSL).
// I just don't know how.

/// Local port to accept incoming connections into
define("LOCALHOST_PORT", "tcp://0.0.0.0:8000");

// MARK:
// Main script
require __DIR__ . "/secure.php";

define("RX_AUTH_FLAG", "A");
define("RX_DROPME_FLAG", "X");
define("TX_AUTH_OK_FLAG", "O");
define("TX_AUTH_FAULT_FLAG", "D");
define("TX_UNKNOWN_FLAG", "U");

$socket = stream_socket_server(LOCALHOST_PORT, $errno, $errstr);

if (!$socket) {
	die("$errstr ($errno)\n");
}

$queue = array();
$approved = array();

while (true) {
	$read = $queue;
	$read [] = $socket;
	$write = $except = null;
	if (!stream_select($read, $write, $except, null)) { break; }
	// New connection
	if (in_array($socket, $read)) {
		// Accept & handshake
		if (($connect = stream_socket_accept($socket, -1)) && $info = handshake($connect)) {
			$queue[] = $connect; // Adding connection to queue
			on_open($connect, $info); // Responding
		}
		unset($read[array_search($socket, $read)]);
	}
	foreach($read as $connect) { // Handle each connection in queue
		$data = fread($connect, 100000);
		if (!strlen($data)) { // Connection closed
			conn_close($connect);
			continue;
		}
		on_message($connect, $data); // Responding to incoming message
	}
}

fclose($server);

// Handling connection opening
function on_open($connect, $info) {
	echo "open\n";
}

// Handling connection closing
function on_close($connect) {
	echo "close\n";
}

// Handling incoming message
function on_message($connect, $data) {
	$response = TX_UNKNOWN_FLAG;
	$txt = decode($data)['payload'];
	$flag = $txt[0];
	$body = substr($txt, 1);
	unset($txt);
	switch ($flag) {
		case RX_AUTH_FLAG:
		$body_exp = explode('/', $body);
		$access_key = $body_exp[0];
		unset($body);
		$user_key = $body_exp[1];
			if ($access_key == ACCESS_KEY && in_array($user_key, USER_KEYS)) {
				$response = TX_AUTH_OK_FLAG;
			} else {
				$response = TX_AUTH_FAULT_FLAG;
			}
			break;
		case RX_DROPME_FLAG:
			echo("dropping");
			conn_close($connect);
			return;
		default:
			echo("unknown command: " . $txt . "\n");
			break;
	}
	fwrite($connect, encode($response));
}

// Dropping the connection
function conn_close($conn) {
	global $queue;
	fclose($conn);
	unset($queue[array_search($conn, $queue)]);
	on_close($conn); // Handling connection closing
}

// This is like quantum physics to me nvm
function handshake($connect) {
	$info = array();
	$line = fgets($connect);
	$header = explode(' ', $line);
	$info['method'] = $header[0];
	$info['uri'] = $header[1];
	while ($line = rtrim(fgets($connect))) {
		if (preg_match('/\A(\S+): (.*)\z/', $line, $matches)) {
			$info[$matches[1]] = $matches[2];
		} else {
			break;
		}
	}
	$address = explode(':', stream_socket_get_name($connect, true));
	$info['ip'] = $address[0];
	$info['port'] = $address[1];
	if (empty($info['Sec-WebSocket-Key'])) { return false; }
	// TODO:
	// Uhhhhh is exposing this key ok? I have no idea.
	$SecWebSocketAccept = base64_encode(pack('H*', sha1($info['Sec-WebSocket-Key'] . '258EAFA5-E914-47DA-95CA-C5AB0DC85B11')));
	$upgrade = "HTTP/1.1 101 Web Socket Protocol Handshake\r\n" .
		"Upgrade: websocket\r\n" .
		"Connection: Upgrade\r\n" .
		"Sec-WebSocket-Accept:$SecWebSocketAccept\r\n\r\n";
	fwrite($connect, $upgrade);
	return $info;
}

// Ok if that one was quantum physics than this is some top-tier sorcery
// (Comments are not mine)
function encode($payload, $type = 'text', $masked = false) {
	$frameHead = array();
	$payloadLength = strlen($payload);
	switch ($type) {
		case 'text':
			// first byte indicates FIN, Text-Frame (10000001):
			$frameHead[0] = 129;
			break;
		case 'close':
			// first byte indicates FIN, Close Frame(10001000):
			$frameHead[0] = 136;
			break;
		case 'ping':
			// first byte indicates FIN, Ping frame (10001001):
			$frameHead[0] = 137;
			break;
		case 'pong':
			// first byte indicates FIN, Pong frame (10001010):
			$frameHead[0] = 138;
			break;
	}
	// set mask and payload length (using 1, 3 or 9 bytes)
	if ($payloadLength > 65535) {
		$payloadLengthBin = str_split(sprintf('%064b', $payloadLength), 8);
		$frameHead[1] = ($masked === true) ? 255 : 127;
		for ($i = 0; $i < 8; $i++) {
			$frameHead[$i + 2] = bindec($payloadLengthBin[$i]);
		}
		// most significant bit MUST be 0
		if ($frameHead[2] > 127) {
			return array('type' => '', 'payload' => '', 'error' => 'frame too large (1004)');
		}
	} else if ($payloadLength > 125) {
		$payloadLengthBin = str_split(sprintf('%016b', $payloadLength), 8);
		$frameHead[1] = ($masked === true) ? 254 : 126;
		$frameHead[2] = bindec($payloadLengthBin[0]);
		$frameHead[3] = bindec($payloadLengthBin[1]);
	} else {
		$frameHead[1] = ($masked === true) ? $payloadLength + 128 : $payloadLength;
	}
	// convert frame-head to string:
	foreach (array_keys($frameHead) as $i) {
		$frameHead[$i] = chr($frameHead[$i]);
	}
	if ($masked === true) {
		// generate a random mask:
		$mask = array();
		for ($i = 0; $i < 4; $i++) {
			$mask[$i] = chr(rand(0, 255));
		}

		$frameHead = array_merge($frameHead, $mask);
	}
	$frame = implode('', $frameHead);
	// append payload to frame:
	for ($i = 0; $i < $payloadLength; $i++) {
		$frame .= ($masked === true) ? $payload[$i] ^ $mask[$i % 4] : $payload[$i];
	}
	return $frame;
}

function decode($data) {
	$unmaskedPayload = '';
	$decodedData = array();
	// estimate frame type:
	$firstByteBinary = sprintf('%08b', ord($data[0]));
	$secondByteBinary = sprintf('%08b', ord($data[1]));
	$opcode = bindec(substr($firstByteBinary, 4, 4));
	$isMasked = ($secondByteBinary[0] == '1') ? true : false;
	$payloadLength = ord($data[1]) & 127;
	// unmasked frame is received:
	if (!$isMasked) {
		return array('type' => '', 'payload' => '', 'error' => 'protocol error (1002)');
	}
	switch ($opcode) {
		// text frame:
		case 1:
			$decodedData['type'] = 'text';
			break;
		case 2:
			$decodedData['type'] = 'binary';
			break;
		// connection close frame:
		case 8:
			$decodedData['type'] = 'close';
			break;
		// ping frame:
		case 9:
			$decodedData['type'] = 'ping';
			break;
		// pong frame:
		case 10:
			$decodedData['type'] = 'pong';
			break;
		default:
			return array('type' => '', 'payload' => '', 'error' => 'unknown opcode (1003)');
	}
	if ($payloadLength === 126) {
		$mask = substr($data, 4, 4);
		$payloadOffset = 8;
		$dataLength = bindec(sprintf('%08b', ord($data[2])) . sprintf('%08b', ord($data[3]))) + $payloadOffset;
	} else if ($payloadLength === 127) {
		$mask = substr($data, 10, 4);
		$payloadOffset = 14;
		$tmp = '';
		for ($i = 0; $i < 8; $i++) {
			$tmp .= sprintf('%08b', ord($data[$i + 2]));
		}
		$dataLength = bindec($tmp) + $payloadOffset;
		unset($tmp);
	} else {
		$mask = substr($data, 2, 4);
		$payloadOffset = 6;
		$dataLength = $payloadLength + $payloadOffset;
	}
	/**
	 * We have to check for large frames here. socket_recv cuts at 1024 bytes
	 * so if websocket-frame is > 1024 bytes we have to wait until whole
	 * data is transferd.
	 */
	if (strlen($data) < $dataLength) {
		return false;
	}
	if ($isMasked) {
		for ($i = $payloadOffset; $i < $dataLength; $i++) {
			$j = $i - $payloadOffset;
			if (isset($data[$i])) {
				$unmaskedPayload .= $data[$i] ^ $mask[$j % 4];
			}
		}
		$decodedData['payload'] = $unmaskedPayload;
	} else {
		$payloadOffset = $payloadOffset - 4;
		$decodedData['payload'] = substr($data, $payloadOffset);
	}
	return $decodedData;
}