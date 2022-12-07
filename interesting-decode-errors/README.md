# Intersting finds when iterating decoding

The first png in this directory (begins with 0) is the result of ignoring filtering entirely. The channels being displayed are also wrong. The de-compressed stream stores colors in RGBA (red, green, blue, alpha) while the Windows bitmap API uses BGRA. You can see an angled line of aggregating off-by-one errors caused by the filter byte being entered into the BGRA stream rather than being discarded.

The second png (begins with 1) is the result of only implementing a portion of the filters. You can see the top of the image render correctly and then begin to fail once it reaches a scanline with incorrect filtering. It begins to correct itself once it reaches a scanline with a defined filter, but it never fully corrects as the filter depends on neighboring pixels (which are in an incorrect state). The incorrect pixels streak down and to the right, showing how the filters depend on pixels to the left and above.
