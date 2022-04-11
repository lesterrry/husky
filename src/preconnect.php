<?php
/*************************
COPYRIGHT LESTER COVEY (me@lestercovey.ml),
2022

*************************/

// Insert here anything which would echo 'Ok' if the server daemon is up and running
$res = shell_exec("prettyservice huskyd");
if ($res === "1") {
	echo("Ok");
} else {
	echo("Err");
}