<?php
echo "\nHere is a worker!\n";
$a = [];
for ($i = 0; $i < 1000000; $i++) {
    $a[] = $i;
}
sleep(30);
