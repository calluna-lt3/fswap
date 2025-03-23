maybe fsp



or:
    fsp s [OPTIONS]
    fsp r
    fsp b
    fsp e


consider having short cmd options =>
    i just want:
        fswap [FILE...]
        fswap -r [FILE...]

USAGE

    fswap swap [OPTIONS] [FILE...]
        Swap FILES from src -> working
        -d, --directory
            swaps files in directory

    fswap help
        show some form of this

    fswap begin [SOURCE_DIR] [WORKING_DIR]
        creates fswap files
        source_dir will NEVER be modified

    fswap end [DIR]
        deletes fswap files

    fswap info [DIR]
        displays all information about fswap files in DIR

    fswap revert [OPTIONS] [FILE.....]
        Revert FILES to original state
        -d, --directory
            reverts all files in directory
        -a, --all
            reverts all files


FUCK YOU THIS IS THE USAGE IDC IF ITS NOT READABLE

    fswap [OPTIONS] [FILE...]
        swap FILEs from src -> working

OPTIONS
    -a, --all
        additional option for -r, reverts all files

    -b, --begin [SOURCE_DIR] [WORKING_DIR]
        begins fswap process (creates .fswap file)

    -d, --directory
        swaps files in directory

    -e, --end [DIR]
        ends fswap process (deletes fswap files)

    -h, --help
        shows help message

    -i, --info [DIR]
        displays all information about fswap files in DIR

    -r, --revert [FILE.....]
        revert FILES to original state

    -v, --verbose
        prints a buncha stuff, lotta lotta stuff

    -s, --silent
        ,,,
