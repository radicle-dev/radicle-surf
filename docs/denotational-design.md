This an attempt to use Denotational Design to define the Code Exploration component of the Code
Collaboration library.

Anything that is a `type` is an object that we refer to in our system. Initially they are
opaque and we do not give them meanings right away. Sometimes they are trivial so elide their
meanings.

When we use `type instance` we are giving an object the simplest meaning we can give it. This
is a process so if the meaning can be simpler we can refine that and iterate on the design.

We then list the functionality of our API by laying out the functions that will work with these
objects in our system. The meanings of these functions are denoted using the symbol `μ`. For
example, below we have the object `Repo` and when we say `μ repo` we are saying that we are
accessing the meaning of `Repo`, which in this case is `([Branch], [Tag])`. Thus, the meaning
of the function `getBranches` is taking the `fst` element from the meaning of `Repo`, i.e.
`fst (μ repo)`.

```haskell
-- A Repo is a series of CommitHistory's where the
-- most recent is the head of the list.
type Repo
type instance Repo = [CommitHistory]

-- A series of commits that is named. This encapsulates
-- both a Branch and a Tag
-- This is assuming that the @head [Commit] == latestCommit@
type CommitHistory = (Name, [Commit])

-- A directory is a named path, in this case calling each
-- part of that path a Component.
-- i.e. /home/foo/bar/ ~ [home, foo, bar]
type Directory
type instance Directory = [Component]

-- A File is its location, 'FileName', and commits
type File
type instance File = (FileName, [Commit])

-- A FileName is a full directory path and the name of
-- the file itself
type FileName
type instance FileName = (Directory, Name)

type FileContents
type instance FileContents = Text

type Commit
type instance Commit = (CommitMeta, [Commit], [Change])

type Author

-- Something like a GPG signature
type Signature

type Hash
type instance Hash = Text

type Date
type instance Date = UTCTime

type Message
type instance Message = Text

type Change
-- Constructors of Change - think GADT
AddLineToFile :: FileName -> Location -> FileContents -> Change
RemoveLineFromFile :: FileName -> Location -> Change
MoveFile :: FileName -> FileName -> Change
CreateFile :: FileName -> Change
DeleteFile :: FileName -> Change

-- Prepend a 'CommitHistory' to a 'Repo'
addCommitHistory :: CommitHistory -> Repo -> Repo
μ addCommitHistory history = history : μ repo

-- Retrieve a list of the 'CommitHistory's
getCommitHistories :: Repo -> [CommitHistory]
μ getCommitHistories repo = μ repo

-- Get a list of the commits in the given history
getCommits :: CommitHistory -> [Commit]
μ getCommits history = snd (μ history)

getHistoryTo :: Commit -> CommitHistory -> [Commit]
μ getHistoryTo commit history =
  dropUpTo (/= commit) (getCommits history)

-- dropWhile including breaking element
dropUpTo :: (a -> Bool) -> [a] -> [a]

-- Get a list of the directories in a given history
getHistoryDirectories :: CommitHistory -> [Directory]
μ getHistoryDirectories =
  nub . map fileDirectory . getFiles . getCommits

-- Helper to get a File's Directory
fileDirectory :: File -> Directory
μ fileDirectory file = fst $ fst (μ file)

fileHistory :: File -> [Commit]
μ fileHistory file = snd (μ file)

-- Gets the all files under a Directory.
-- My thinking is that we could cut off the result of this
-- to have a view of the files and the next directories.
getDirectoryView :: Directory -> [File] -> [File]
μ getDirectoryView directory files =
  filter (\file -> directory `isPrefixOf` fileDirectory file) [] files

-- Check that the path of the first directory is a prefix to the
-- path of the second directory
-- i.e. /home/foo `isPrefixOf` /home/foo/bar == True
--      forall d. d `isPrefixOf` d == True
isPrefixOf :: Directory -> Directory -> Bool
μ isPrefixOf prefix directory = μ prefix `List.isPrefixOf` μ directory

-- Get the Files for this set of commits
getFiles :: [Commit] -> [File]
μ getFiles commits =
  concatMap assocs $
    foldr (\commit -> foldr (\change -> union (buildChangeMap commit change)) mapEmpty (getCommitChanges commit))
          []
          commits

-- Keep track of how a file changes within a commit
buildChangeMap :: Commit -> Change -> Map FileName [Commit] -> Map FileName [Commit]
μ buildChangeMap commit change changeMap = case change of
  CreateFile filename             -> insertWith filename (commit:) changeMap
  DeleteFile filename             -> delete filename changeMap
  MoveFile (filename, filename')  -> insertWith filename' (commit:)
                                   . changeKey filename filename'
                                   $ changeMap
  AddLineToFile filname _ _       -> insertWith filename (commit:) changeMap
  RemoveLineFromFile filename _ _ -> insertWith filename (commit:) changeMap

directoryHistory :: CommitHistory -> Directory -> [Commit]
μ directoryHistory history directory =
  foldMap fileHistory (getDirectoryView directory (getFiles history))

-- To get FileContents for a certain commit we can do:
-- @fileContentsUpTo file (getHistoryTo commit)@
fileContentsUpTo :: File -> [Commit] -> FileContents
μ fileContentsUpTo file commits =
  foldr applyChange emptyFileContents
  . filter (\change -> changeFile change == fst (μ file))
  $ commits

-- See the up-to-date view of a File
currentFileContents :: File -> FileContents
μ currentFileContents file =
  fileContentsUpTo file . foldMap getCommitChanges $ snd (μ file)

applyChange :: Change -> FileContents -> FileContents
μ applyChange change fileContents = case change of
  AddLineToFile _ location fileContents' -> addLine location fileContents' fileContents
  RemoveLineFromFile _ location          -> removeLine location fileContents
  -- These operations don't modify contents per se but rather just modify the file
  MoveFile _ _                           -> fileContents
  CreateFile _                           -> fileContents
  -- Deleting will set FileContents to nothing
  DeleteFile _                           -> emptyFileContents

fileName :: File -> FileName
μ fileName file = fst (μ file)

-- Eliding details because they should be trivial getters on Commit metadata
commitAuthor :: Commit -> Author
commitHash :: Commit -> Hash
commitDate :: Commit -> Date
commitMessage :: Commit -> Message
commitSignature :: Commit -> Maybe Signature

signCommit :: Key -> Commit -> Commit
μ signCommit key commit = μ commit { signature = sign key }

commitParents :: Commit -> [Commit]
μ commitParents commit = snd (μ commit)

commitChildren :: Repo -> Commit -> [Commit]
μ commitChildren repo commit =
  foldMap (dropWhile (/= commit) . getCommits) (μ repo)

findAuthorCommits :: Author -> [Commit] -> [Commit]
μ findAuthorCommits author commits = filter ((== author) . commitAuthor) commits

getCommitByHash :: Hash -> CommitHistory -> Maybe Commit
μ getCommitByHash hash history = find ((== hash) . commitHash) (snd $ μ history)

getRepoCommit :: Hash -> Repo -> Maybe Commit
μ getRepoCommit hash repo = findMaybe (getCommitByHash hash) (getCommitHistories repo)

findMaybe :: (a -> Maybe b) -> [a] -> Maybe b

getCommitChanges :: Commit -> [Change]
μ getCommitChanges commit = third id (μ commit)

-- Properties

-- If the File's commits are equivalent to the commits in the CommitHistory
-- this implies that the File should be an element of the File's we can build
-- from the CommitHistory.
file_must_exist_in_history :: Property
file_must_exist_in_history = ∀ file history. fileHistory file ≡ getCommits history ⇒ file ∈ getFiles (getCommits history)

-- Building a File from from its own Commits should result back in the file
-- itself.
file_is_its_history :: Property
file_is_its_history = ∀ file. getFiles (fileHistory file) ≡ [file]

-- Getting the [Directory] from CommitHistory should be equivalent to building
-- the [File] and getting their Directory
files_and_directories :: Property
files_and_directories = ∀ history. getHistoryDirectories history ≡ map fileDirectory (getFiles (getCommits history))
```
