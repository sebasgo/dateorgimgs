dateorgimgs
===========

Rename image files in a given folder according to their EXIV date
information and camera model. I use it store image files from multiple
sources in single folder with a natural sort order.

Usage
-----


    dateorgimgs [--prefix PREFIX] [--dryrun] [PATH]


All files in PATH are scanned for EXIF information and will be renamed
to `INDEX YY-MM-DD HH-MM-SS CAMERA`, e.g. `201 2012-03-04 18-42-32 NIKON
D7100`. With `--prefix` one can set custom file name prefix. `--dryrun`
allows one to preview the changes without actually performing them.

Copyright & License
-------------------

This program is free software: you can redistribute it and/or modify it
under the terms of the GNU General Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your option)
any later version.

This program is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with
this program. If not, see <http://www.gnu.org/licenses/>.

Please refer to the [`LICENSE`](LICENSE) file for a full copy of the license.
