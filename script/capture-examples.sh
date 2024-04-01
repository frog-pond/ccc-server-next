#!/bin/sh

	# "stav-hall" => 261,
	# "the-cage" => 262,
	# "kings-room" => 263,
	# "burton" => 35,
	# "ldc" => 36,
	# "sayles" => 24,
	# "weitz" => 458,

EXAMPLES_DIR="$(dirname $0)/../ccc-bonapp/src/bonapp-examples"

# Capture example cafe responses

STAV_HALL_ID=261
THE_CAGE_ID=262
KINGS_ROOM_ID=263
BURTON_ID=35
LDC_ID=36
SAYLES_ID=24
WEITZ_ID=458

for cafe_id in $STAV_HALL_ID $THE_CAGE_ID $KINGS_ROOM_ID $BURTON_ID $LDC_ID $SAYLES_ID $WEITZ_ID;
do
	url="https://legacy.cafebonappetit.com/api/2/cafes?cafe=${cafe_id}"
	filename="$EXAMPLES_DIR/$(date '+%s')--cafe-${cafe_id}--$(echo -n $url | base64).json"

	echo "GET $url ~> $filename"

	curl -s "https://legacy.cafebonappetit.com/api/2/cafes?cafe=${cafe_id}" > "$filename"
done
